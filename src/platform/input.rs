//! Cross-platform input simulation built on `enigo`.

use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::key::KeyChord;
use crate::core::macro_engine::MacroSink;
use crate::platform::error::PlatformError;
use crate::platform::key_map::{to_enigo_key, to_enigo_modifiers};
use enigo::{Axis, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::path::Path;

/// Sink used to synthesize keyboard, mouse and media input.
pub trait InputSink: Send {
    /// Presses or releases a chord.
    fn send_chord(&mut self, chord: KeyChord, down: bool) -> Result<(), PlatformError>;

    /// Presses and releases a chord.
    fn tap_chord(&mut self, chord: KeyChord) -> Result<(), PlatformError>;

    /// Types text.
    fn type_text(&mut self, text: &str) -> Result<(), PlatformError>;

    /// Scrolls the window under the cursor.
    fn scroll(&mut self, axis: ScrollAxis, amount: i32) -> Result<(), PlatformError>;

    /// Clicks a mouse button.
    fn mouse_click(&mut self, button: MouseButton) -> Result<(), PlatformError>;

    /// Changes the system volume by OS steps.
    fn volume(&mut self, direction: VolumeDirection, steps: u8) -> Result<(), PlatformError>;

    /// Changes the screen brightness by OS steps.
    fn brightness(&mut self, direction: VolumeDirection, steps: u8) -> Result<(), PlatformError>;

    /// Sends a media transport command.
    fn media(&mut self, command: MediaCommand) -> Result<(), PlatformError>;

    /// Launches an application or file.
    fn launch(&mut self, path: &Path) -> Result<(), PlatformError>;
}

/// Input sink backed by `enigo`.
pub struct EnigoInputSink {
    enigo: Enigo,
}

impl EnigoInputSink {
    /// Creates the sink. Platform permissions may be required.
    pub fn new() -> Result<Self, PlatformError> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|error| PlatformError::Other(error.to_string()))?;
        Ok(Self { enigo })
    }
}

impl InputSink for EnigoInputSink {
    fn send_chord(&mut self, chord: KeyChord, down: bool) -> Result<(), PlatformError> {
        let key = to_enigo_key(chord.key).ok_or(PlatformError::Unsupported(
            "key code is not supported by the input sink",
        ))?;
        let modifiers = to_enigo_modifiers(chord.modifiers);

        if down {
            for modifier in &modifiers {
                self.enigo.key(*modifier, Direction::Press)?;
            }
            self.enigo.key(key, Direction::Press)?;
        } else {
            self.enigo.key(key, Direction::Release)?;
            for modifier in modifiers.iter().rev() {
                self.enigo.key(*modifier, Direction::Release)?;
            }
        }

        Ok(())
    }

    fn tap_chord(&mut self, chord: KeyChord) -> Result<(), PlatformError> {
        let key = to_enigo_key(chord.key).ok_or(PlatformError::Unsupported(
            "key code is not supported by the input sink",
        ))?;
        let modifiers = to_enigo_modifiers(chord.modifiers);

        for modifier in &modifiers {
            self.enigo.key(*modifier, Direction::Press)?;
        }
        self.enigo.key(key, Direction::Click)?;
        for modifier in modifiers.iter().rev() {
            self.enigo.key(*modifier, Direction::Release)?;
        }

        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<(), PlatformError> {
        self.enigo.text(text)?;
        Ok(())
    }

    fn scroll(&mut self, axis: ScrollAxis, amount: i32) -> Result<(), PlatformError> {
        let axis = match axis {
            ScrollAxis::Vertical => Axis::Vertical,
            ScrollAxis::Horizontal => Axis::Horizontal,
        };
        self.enigo.scroll(amount, axis)?;
        Ok(())
    }

    fn mouse_click(&mut self, button: MouseButton) -> Result<(), PlatformError> {
        let button = match button {
            MouseButton::Left => enigo::Button::Left,
            MouseButton::Right => enigo::Button::Right,
            MouseButton::Middle => enigo::Button::Middle,
        };
        self.enigo.button(button, Direction::Click)?;
        Ok(())
    }

    fn volume(&mut self, direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
        let key = match direction {
            VolumeDirection::Up => Key::VolumeUp,
            VolumeDirection::Down => Key::VolumeDown,
        };
        for _ in 0..steps {
            self.enigo.key(key, Direction::Click)?;
        }
        Ok(())
    }

    fn brightness(&mut self, direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let key = match direction {
                VolumeDirection::Up => Key::BrightnessUp,
                VolumeDirection::Down => Key::BrightnessDown,
            };
            for _ in 0..steps {
                self.enigo.key(key, Direction::Click)?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (direction, steps);
            Err(PlatformError::Unsupported(
                "screen brightness is not available on this platform",
            ))
        }
    }

    fn media(&mut self, command: MediaCommand) -> Result<(), PlatformError> {
        let key = match command {
            MediaCommand::PlayPause => Key::MediaPlayPause,
            MediaCommand::Next => Key::MediaNextTrack,
            MediaCommand::Previous => Key::MediaPrevTrack,
            #[cfg(not(target_os = "macos"))]
            MediaCommand::Stop => Key::MediaStop,
            #[cfg(target_os = "macos")]
            MediaCommand::Stop => {
                return Err(PlatformError::Unsupported(
                    "media stop is not available on this platform",
                ));
            }
        };
        self.enigo.key(key, Direction::Click)?;
        Ok(())
    }

    fn launch(&mut self, path: &Path) -> Result<(), PlatformError> {
        launch_path(path)
    }
}

impl MacroSink for EnigoInputSink {
    fn tap(&mut self, chord: KeyChord) -> Result<(), crate::core::macro_engine::MacroError> {
        self.tap_chord(chord).map_err(to_macro_error)
    }

    fn press(&mut self, chord: KeyChord) -> Result<(), crate::core::macro_engine::MacroError> {
        self.send_chord(chord, true).map_err(to_macro_error)
    }

    fn release(&mut self, chord: KeyChord) -> Result<(), crate::core::macro_engine::MacroError> {
        self.send_chord(chord, false).map_err(to_macro_error)
    }

    fn text(&mut self, text: &str) -> Result<(), crate::core::macro_engine::MacroError> {
        self.type_text(text).map_err(to_macro_error)
    }

    fn scroll(
        &mut self,
        axis: ScrollAxis,
        amount: i32,
    ) -> Result<(), crate::core::macro_engine::MacroError> {
        InputSink::scroll(self, axis, amount).map_err(to_macro_error)
    }

    fn mouse_click(
        &mut self,
        button: MouseButton,
    ) -> Result<(), crate::core::macro_engine::MacroError> {
        InputSink::mouse_click(self, button).map_err(to_macro_error)
    }
}

fn to_macro_error(error: PlatformError) -> crate::core::macro_engine::MacroError {
    crate::core::macro_engine::MacroError::Sink(error.to_string())
}

fn launch_path(path: &Path) -> Result<(), PlatformError> {
    #[cfg(target_os = "macos")]
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
    {
        std::process::Command::new("open")
            .arg("-a")
            .arg(path)
            .spawn()?;
        return Ok(());
    }

    std::process::Command::new(path).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn launching_a_missing_path_fails() {
        let missing = PathBuf::from("/definitely/not/a/real/application");
        assert!(launch_path(&missing).is_err());
    }
}
