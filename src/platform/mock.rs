//! Test doubles for the platform adapters.

use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::gesture::InputEvent;
use crate::core::key::KeyChord;
use crate::core::macro_engine::MacroError;
use crate::platform::error::PlatformError;
use crate::platform::hotkeys::HotkeySource;
use crate::platform::input::InputSink;
use std::path::Path;

// Re-exported for the `MacroSink` impl below.
use crate::core::macro_engine::MacroSink;

/// Hotkey source that records applied chords and queued events.
#[derive(Default)]
pub struct MockHotkeys {
    /// Chords currently registered.
    pub chords: Vec<KeyChord>,
    /// Events returned by [`MockHotkeys::poll`].
    pub events: Vec<InputEvent>,
    /// When true, [`HotkeySource::apply`] fails.
    pub fail_apply: bool,
}

impl HotkeySource for MockHotkeys {
    fn apply(&mut self, chords: &[KeyChord]) -> Result<(), PlatformError> {
        if self.fail_apply {
            return Err(PlatformError::Other("mock hotkey failure".to_owned()));
        }
        self.chords = chords.to_vec();
        Ok(())
    }

    fn unregister_all(&mut self) -> Result<(), PlatformError> {
        self.chords.clear();
        Ok(())
    }

    fn poll(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Input sink that records calls as strings.
#[derive(Default)]
pub struct MockInputSink {
    /// Recorded calls.
    pub calls: Vec<String>,
    /// When true, every call fails.
    pub fail: bool,
}

impl MockInputSink {
    fn record(&mut self, call: String) -> Result<(), PlatformError> {
        if self.fail {
            return Err(PlatformError::Other("mock input failure".to_owned()));
        }
        self.calls.push(call);
        Ok(())
    }
}

impl InputSink for MockInputSink {
    fn send_chord(&mut self, chord: KeyChord, down: bool) -> Result<(), PlatformError> {
        let direction = if down { "down" } else { "up" };
        self.record(format!("chord {direction} {chord}"))
    }

    fn tap_chord(&mut self, chord: KeyChord) -> Result<(), PlatformError> {
        self.record(format!("tap {chord}"))
    }

    fn type_text(&mut self, text: &str) -> Result<(), PlatformError> {
        self.record(format!("text {text}"))
    }

    fn scroll(&mut self, axis: ScrollAxis, amount: i32) -> Result<(), PlatformError> {
        self.record(format!("scroll {axis:?} {amount}"))
    }

    fn mouse_click(&mut self, button: MouseButton) -> Result<(), PlatformError> {
        self.record(format!("click {button:?}"))
    }

    fn volume(&mut self, direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
        self.record(format!("volume {direction:?} {steps}"))
    }

    fn media(&mut self, command: MediaCommand) -> Result<(), PlatformError> {
        self.record(format!("media {command:?}"))
    }

    fn launch(&mut self, path: &Path) -> Result<(), PlatformError> {
        self.record(format!("launch {}", path.display()))
    }
}

impl MacroSink for MockInputSink {
    fn tap(&mut self, chord: KeyChord) -> Result<(), MacroError> {
        self.tap_chord(chord).map_err(to_macro_error)
    }

    fn press(&mut self, chord: KeyChord) -> Result<(), MacroError> {
        self.send_chord(chord, true).map_err(to_macro_error)
    }

    fn release(&mut self, chord: KeyChord) -> Result<(), MacroError> {
        self.send_chord(chord, false).map_err(to_macro_error)
    }

    fn text(&mut self, text: &str) -> Result<(), MacroError> {
        self.type_text(text).map_err(to_macro_error)
    }

    fn scroll(&mut self, axis: ScrollAxis, amount: i32) -> Result<(), MacroError> {
        InputSink::scroll(self, axis, amount).map_err(to_macro_error)
    }

    fn mouse_click(&mut self, button: MouseButton) -> Result<(), MacroError> {
        InputSink::mouse_click(self, button).map_err(to_macro_error)
    }
}

fn to_macro_error(error: PlatformError) -> MacroError {
    MacroError::Sink(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::key::KeyCode;

    #[test]
    fn mock_hotkeys_record_chords() {
        let mut hotkeys = MockHotkeys::default();
        let chords = vec![KeyChord::new(KeyCode::F13)];
        hotkeys.apply(&chords).expect("apply");
        assert_eq!(hotkeys.chords, chords);
        hotkeys.unregister_all().expect("clear");
        assert!(hotkeys.chords.is_empty());
    }

    #[test]
    fn mock_input_records_calls_and_can_fail() {
        let mut sink = MockInputSink::default();
        sink.tap_chord(KeyChord::new(KeyCode::F13)).expect("tap");
        sink.volume(VolumeDirection::Up, 2).expect("volume");
        assert_eq!(sink.calls.len(), 2);

        sink.fail = true;
        assert!(sink.tap_chord(KeyChord::new(KeyCode::F14)).is_err());
    }
}
