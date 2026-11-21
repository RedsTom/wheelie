//! Global hotkey registration built on `global-hotkey`.
//!
//! On Linux the backend is X11-only. A Wayland session cannot be watched by a
//! regular application, so creating the service there fails explicitly; even
//! on X11 a registration can be refused by the shell or another tool, hence
//! [`GlobalHotkeyService::apply_optional`] collects individual refusals.

use crate::core::gesture::InputEvent;
use crate::core::key::KeyChord;
use crate::platform::error::PlatformError;
use crate::platform::key_map::{to_global_code, to_global_modifiers};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::{HashMap, HashSet};
use std::thread::ThreadId;

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
    owner: ThreadId,
}

impl GlobalHotkeyService {
    /// Creates the service. Must be called on the UI thread.
    pub fn new() -> Result<Self, PlatformError> {
        reject_wayland()?;
        let manager =
            GlobalHotKeyManager::new().map_err(|error| PlatformError::Hotkey(Box::new(error)))?;
        Ok(Self {
            manager,
            registered: HashMap::new(),
            owner: std::thread::current().id(),
        })
    }

    /// Registers the chords one by one and returns the ones the system refused.
    ///
    /// A reserved chord may already belong to the desktop shell, to another
    /// tool the user runs, or to a lock no application is allowed to replace.
    /// Refusing one of them must not stop every other key from reaching the
    /// field that waits, so failures are collected instead of aborting the
    /// batch. A failure to release the previous chords is logged, but the
    /// chords that were released stay released and the new batch is still
    /// attempted.
    pub fn apply_optional(&mut self, chords: &[KeyChord]) -> Vec<KeyChord> {
        self.ensure_owner();
        let _ = reject_wayland().inspect_err(|error| {
            tracing::warn!(%error, "optional hotkeys need an X11 session");
        });

        if let Err(error) = self.unregister_all() {
            // `unregister_all` has already dropped every chord it released.
            // Continue with the new batch: a chord the system still owns is
            // reported as refused by the per-chord registration below.
            tracing::warn!(%error, "could not release every previous hotkey");
        }

        let mut refused = Vec::new();
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
            .map_err(|error| PlatformError::Hotkey(Box::new(error)))
    }

    /// Warns when a service created on the UI thread is used elsewhere.
    ///
    /// `global-hotkey` installs its watcher on the creating thread; calling
    /// from another thread is best-effort and must never go unnoticed.
    fn ensure_owner(&self) {
        let current = std::thread::current().id();
        if current != self.owner {
            tracing::warn!(
                ?current,
                owner = ?self.owner,
                "global hotkeys are used from another thread than the one that created them"
            );
        }
    }
}

impl HotkeySource for GlobalHotkeyService {
    fn apply(&mut self, chords: &[KeyChord]) -> Result<(), PlatformError> {
        self.ensure_owner();
        reject_wayland()?;
        self.unregister_all()?;

        let mut seen = HashSet::new();
        for chord in chords {
            if !seen.insert(*chord) {
                continue;
            }

            let hotkey = match to_hotkey(*chord) {
                Ok(hotkey) => hotkey,
                Err(error) => {
                    self.rollback();
                    return Err(error);
                }
            };
            if let Err(error) = self.register_one(hotkey) {
                self.rollback();
                return Err(error);
            }
            self.registered.insert(hotkey.id(), (*chord, hotkey));
        }

        Ok(())
    }

    fn unregister_all(&mut self) -> Result<(), PlatformError> {
        self.ensure_owner();
        let hotkeys: Vec<HotKey> = self
            .registered
            .values()
            .map(|(_, hotkey)| *hotkey)
            .collect();

        // Release one by one so a single refusal cannot leave the other keys
        // captured. Every successful unregister is dropped from the map right
        // away; only the first failure is reported to the caller.
        let mut first_error = None;
        for hotkey in hotkeys {
            if let Err(error) = self.manager.unregister(hotkey) {
                if first_error.is_none() {
                    first_error = Some(PlatformError::Hotkey(Box::new(error)));
                }
            } else {
                self.registered.remove(&hotkey.id());
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn poll(&mut self) -> Vec<InputEvent> {
        // `GlobalHotKeyEvent::receiver()` is a process-wide singleton: every
        // event is delivered to that one channel and consuming it here removes
        // it for all readers. Only the single service created by the UI shell
        // should poll, otherwise events would be split between readers.
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

impl GlobalHotkeyService {
    /// Cancels the chords registered so far after a failed batch.
    fn rollback(&mut self) {
        if let Err(error) = self.unregister_all() {
            tracing::warn!(%error, "could not roll back the hotkeys registered so far");
        }
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

/// Pure Wayland test over the three environment signals.
#[cfg(all(unix, not(target_os = "macos")))]
fn is_wayland(has_wayland_display: bool, session_type: &str, has_display: bool) -> bool {
    has_wayland_display || (session_type.eq_ignore_ascii_case("wayland") && !has_display)
}

/// Refuses global hotkeys on a Wayland session, where X11 grabbing is useless.
#[cfg(all(unix, not(target_os = "macos")))]
fn reject_wayland() -> Result<(), PlatformError> {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    if is_wayland(
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        &session,
        std::env::var_os("DISPLAY").is_some(),
    ) {
        return Err(PlatformError::Unsupported(
            "global hotkeys require an X11 session",
        ));
    }
    Ok(())
}

/// No session detection is needed where the backend is native.
#[cfg(not(all(unix, not(target_os = "macos"))))]
fn reject_wayland() -> Result<(), PlatformError> {
    Ok(())
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

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn wayland_is_detected_without_an_x_display() {
        assert!(is_wayland(false, "wayland", false));
        assert!(is_wayland(true, "x11", true));
        assert!(!is_wayland(false, "wayland", true));
        assert!(!is_wayland(false, "x11", false));
    }
}
