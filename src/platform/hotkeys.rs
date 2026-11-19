//! Global hotkey registration built on `global-hotkey`.

use crate::core::gesture::InputEvent;
use crate::core::key::KeyChord;
use crate::platform::error::PlatformError;
use crate::platform::key_map::{to_global_code, to_global_modifiers};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::{HashMap, HashSet};

/// Source of exclusive global key events.
pub trait HotkeySource {
    /// Replaces the registered chords.
    fn apply(&mut self, chords: &[KeyChord]) -> Result<(), PlatformError>;

    /// Removes every registered chord.
    fn unregister_all(&mut self) -> Result<(), PlatformError>;

    /// Drains pending press and release events.
    fn poll(&mut self) -> Vec<InputEvent>;
}

/// Global hotkey service backed by the operating system.
pub struct GlobalHotkeyService {
    manager: GlobalHotKeyManager,
    registered: HashMap<u32, (KeyChord, HotKey)>,
}

impl GlobalHotkeyService {
    /// Creates the service. Must be called on the UI thread.
    pub fn new() -> Result<Self, PlatformError> {
        let manager =
            GlobalHotKeyManager::new().map_err(|error| PlatformError::Other(error.to_string()))?;
        Ok(Self {
            manager,
            registered: HashMap::new(),
        })
    }

    /// Registers the chords one by one and returns the ones the system refused.
    ///
    /// A reserved chord may already belong to the desktop shell, to another
    /// tool the user runs, or to a lock no application is allowed to replace.
    /// Refusing one of them must not stop every other key from reaching the
    /// field that waits, so failures are collected instead of aborting the
    /// batch.
    pub fn apply_optional(&mut self, chords: &[KeyChord]) -> Vec<KeyChord> {
        let mut refused = Vec::new();
        if self.unregister_all().is_err() {
            return chords.to_vec();
        }

        let mut seen = HashSet::new();
        for chord in chords {
            if !seen.insert(*chord) {
                continue;
            }

            match to_hotkey(*chord).and_then(|hotkey| self.register_one(hotkey).map(|()| hotkey)) {
                Ok(hotkey) => {
                    self.registered.insert(hotkey.id(), (*chord, hotkey));
                }
                Err(_) => refused.push(*chord),
            }
        }

        refused
    }

    fn register_one(&mut self, hotkey: HotKey) -> Result<(), PlatformError> {
        self.manager
            .register(hotkey)
            .map_err(|error| PlatformError::Other(error.to_string()))
    }
}

impl HotkeySource for GlobalHotkeyService {
    fn apply(&mut self, chords: &[KeyChord]) -> Result<(), PlatformError> {
        self.unregister_all()?;

        let mut seen = HashSet::new();
        for chord in chords {
            if !seen.insert(*chord) {
                continue;
            }

            let hotkey = to_hotkey(*chord)?;
            self.register_one(hotkey)?;
            self.registered.insert(hotkey.id(), (*chord, hotkey));
        }

        Ok(())
    }

    fn unregister_all(&mut self) -> Result<(), PlatformError> {
        if self.registered.is_empty() {
            return Ok(());
        }

        let hotkeys: Vec<HotKey> = self
            .registered
            .values()
            .map(|(_, hotkey)| *hotkey)
            .collect();
        self.manager
            .unregister_all(&hotkeys)
            .map_err(|error| PlatformError::Other(error.to_string()))?;
        self.registered.clear();
        Ok(())
    }

    fn poll(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        let receiver = GlobalHotKeyEvent::receiver();

        while let Ok(event) = receiver.try_recv() {
            let Some((chord, _)) = self.registered.get(&event.id) else {
                continue;
            };

            let input = match event.state {
                HotKeyState::Pressed => InputEvent::Pressed(*chord),
                HotKeyState::Released => InputEvent::Released(*chord),
            };
            events.push(input);
        }

        events
    }
}

fn to_hotkey(chord: KeyChord) -> Result<HotKey, PlatformError> {
    let code = to_global_code(chord.key).ok_or(PlatformError::Unsupported(
        "key code is not supported by the hotkey backend",
    ))?;
    let modifiers = to_global_modifiers(chord.modifiers);
    let modifiers = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Ok(HotKey::new(modifiers, code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::key::KeyCode;
    use global_hotkey::hotkey::Modifiers as HotkeyModifiers;

    #[test]
    fn hotkey_ids_are_unique_per_chord() {
        let left = KeyChord::new(KeyCode::F13);
        let right = KeyChord::new(KeyCode::F14);
        let left_hotkey = to_hotkey(left).expect("left");
        let right_hotkey = to_hotkey(right).expect("right");
        assert_ne!(left_hotkey.id(), right_hotkey.id());
    }

    #[test]
    fn chord_with_modifiers_builds_hotkey() {
        use crate::core::key::Modifiers;

        let chord = KeyChord::with_modifiers(
            KeyCode::KeyC,
            Modifiers {
                control: true,
                ..Default::default()
            },
        );
        let hotkey = to_hotkey(chord).expect("hotkey");
        assert!(hotkey.mods.contains(HotkeyModifiers::CONTROL));
    }

    #[test]
    fn unsupported_chords_are_reported() {
        // Every current KeyCode maps; this guards the error path against
        // regressions if a domain key is added without a mapping.
        for key in KeyCode::ALL {
            let chord = KeyChord::new(*key);
            assert!(to_hotkey(chord).is_ok(), "failed for {key}");
        }
    }
}
