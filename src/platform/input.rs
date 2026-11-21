//! Cross-platform input simulation built on `enigo`.

use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::key::{KeyChord, KeyCode};
use crate::core::macro_engine::MacroSink;
#[cfg(not(target_os = "macos"))]
use crate::platform::brightness;
use crate::platform::error::PlatformError;
use crate::platform::key_map::{to_enigo_key, to_enigo_modifiers};
use crate::platform::volume;
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

    /// Changes the screen brightness by OS steps. Returns the new level in
    /// percent when the OS reports it.
    fn brightness(
        &mut self,
        direction: VolumeDirection,
        steps: u8,
    ) -> Result<Option<u8>, PlatformError>;

    /// Toggles the system mute state.
    fn volume_mute(&mut self) -> Result<(), PlatformError>;

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

/// Modifiers held while a chord is being injected.
///
/// The drop path always walks the keys in reverse press order and keeps going
/// after a failed release, so a partial failure cannot leave a sticky Ctrl or
/// Alt. The first failure is returned by [`PressedModifiers::release`] when
/// the operation completes normally; an early drop only logs it.
struct PressedModifiers<'a> {
    enigo: &'a mut Enigo,
    keys: Vec<Key>,
}

impl<'a> PressedModifiers<'a> {
    fn press(enigo: &'a mut Enigo, keys: Vec<Key>) -> Result<Self, PlatformError> {
        for (index, &key) in keys.iter().enumerate() {
            if let Err(error) = enigo.key(key, Direction::Press) {
                let _ = release_modifiers(enigo, &keys[..index]);
                return Err(PlatformError::Input(error));
            }
        }
        Ok(Self { enigo, keys })
    }

    /// Releases every held modifier and reports the first failure.
    fn release(mut self) -> Result<(), PlatformError> {
        let result = release_modifiers(&mut *self.enigo, &self.keys);
        self.keys.clear();
        result
    }

    /// Runs an injection while the modifiers are held.
    ///
    /// Modifiers are released afterwards unless `hold` is set and the action
    /// succeeded, which keeps a press paired with a later release. The action
    /// error wins over a release error; the guard's `Drop` is the last resort
    /// if this method is never reached.
    fn run(
        mut self,
        hold: bool,
        action: impl FnOnce(&mut Enigo) -> Result<(), enigo::InputError>,
    ) -> Result<(), PlatformError> {
        let result = action(&mut *self.enigo);
        if hold && result.is_ok() {
            self.keys.clear();
            return Ok(());
        }
        let released = self.release();
        result?;
        released
    }
}

impl Drop for PressedModifiers<'_> {
    fn drop(&mut self) {
        if self.keys.is_empty() {
            return;
        }
        if let Err(error) = release_modifiers(&mut *self.enigo, &self.keys) {
            tracing::warn!(%error, "could not release chord modifiers");
        }
    }
}

/// Releases modifiers from the last pressed to the first, keeping the first error.
fn release_modifiers(enigo: &mut Enigo, keys: &[Key]) -> Result<(), PlatformError> {
    let mut first_error = None;
    for &key in keys.iter().rev() {
        if let Err(error) = enigo.key(key, Direction::Release)
            && first_error.is_none()
        {
            first_error = Some(PlatformError::Input(error));
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

impl InputSink for EnigoInputSink {
    fn send_chord(&mut self, chord: KeyChord, down: bool) -> Result<(), PlatformError> {
        let key = to_enigo_key(chord.key).ok_or(PlatformError::Unsupported(
            "key code is not supported by the input sink",
        ))?;
        let modifiers = to_enigo_modifiers(chord.modifiers);

        if down {
            PressedModifiers::press(&mut self.enigo, modifiers)?
                .run(true, |enigo| enigo.key(key, Direction::Press))?;
        } else {
            let key_result = self.enigo.key(key, Direction::Release);
            let modifiers_result = release_modifiers(&mut self.enigo, &modifiers);
            key_result?;
            modifiers_result?;
        }

        Ok(())
    }

    fn tap_chord(&mut self, chord: KeyChord) -> Result<(), PlatformError> {
        let key = to_enigo_key(chord.key).ok_or(PlatformError::Unsupported(
            "key code is not supported by the input sink",
        ))?;
        let modifiers = to_enigo_modifiers(chord.modifiers);

        PressedModifiers::press(&mut self.enigo, modifiers)?
            .run(false, |enigo| enigo.key(key, Direction::Click))
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
        volume::adjust(direction, steps)
    }

    fn brightness(
        &mut self,
        direction: VolumeDirection,
        steps: u8,
    ) -> Result<Option<u8>, PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let key = match direction {
                VolumeDirection::Up => Key::BrightnessUp,
                VolumeDirection::Down => Key::BrightnessDown,
            };
            for _ in 0..steps {
                self.enigo.key(key, Direction::Click)?;
            }
            Ok(None)
        }
        #[cfg(not(target_os = "macos"))]
        {
            brightness::adjust(direction, steps)
        }
    }

    fn volume_mute(&mut self) -> Result<(), PlatformError> {
        volume::toggle_mute()
    }

    fn media(&mut self, command: MediaCommand) -> Result<(), PlatformError> {
        let key = media_key(command)?;
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

/// Domain key behind a transport command; also feeds the platform key map.
fn media_domain_key(command: MediaCommand) -> KeyCode {
    match command {
        MediaCommand::PlayPause => KeyCode::MediaPlayPause,
        MediaCommand::Next => KeyCode::MediaNext,
        MediaCommand::Previous => KeyCode::MediaPrevious,
        MediaCommand::Stop => KeyCode::MediaStop,
    }
}

/// Resolves a transport command through the key map, which is the single
/// source of truth for platform limits (MediaStop is absent on macOS).
fn media_key(command: MediaCommand) -> Result<Key, PlatformError> {
    to_enigo_key(media_domain_key(command)).ok_or(PlatformError::Unsupported(
        "this media command is not available on this platform",
    ))
}

fn launch_path(path: &Path) -> Result<(), PlatformError> {
    #[cfg(target_os = "windows")]
    return launch_windows(path);

    #[cfg(all(unix, not(target_os = "macos")))]
    return launch_unix(path);

    #[cfg(target_os = "macos")]
    return launch_macos(path);

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = path;
        Err(PlatformError::Unsupported(
            "launching applications is not available on this platform",
        ))
    }
}

/// Opens a shortcut through the shell, which resolves the target itself.
#[cfg(target_os = "windows")]
fn launch_windows(path: &Path) -> Result<(), PlatformError> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{PCWSTR, w};

    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        let target = windows::core::HSTRING::from(path);
        // SAFETY: `w!("open")` and `target` are live for the whole call and
        // `ShellExecuteW` only reads them; null optional parameters are valid.
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                &target,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(PlatformError::Other(format!(
                "the shell refused to open {}",
                path.display()
            )));
        }
        return Ok(());
    }

    let child = std::process::Command::new(path).spawn()?;
    drop(child); // deliberate detach: an action must not wait for the app to exit
    Ok(())
}

/// Uses the desktop-entry-aware launchers where available.
#[cfg(all(unix, not(target_os = "macos")))]
fn launch_unix(path: &Path) -> Result<(), PlatformError> {
    use std::process::Command;

    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
    {
        let text = path.to_string_lossy();
        if let Ok(status) = Command::new("gio").args(["launch", text.as_ref()]).status() {
            if status.success() {
                return Ok(());
            }
        }
        let status = Command::new("xdg-open").arg(path).status()?;
        if !status.success() {
            return Err(PlatformError::Other(format!(
                "the desktop entry could not be opened: {status}"
            )));
        }
        return Ok(());
    }

    let child = Command::new(path).spawn()?;
    drop(child); // deliberate detach: an action must not wait for the app to exit
    Ok(())
}

/// `open` handles applications, documents and folders alike on macOS.
#[cfg(target_os = "macos")]
fn launch_macos(path: &Path) -> Result<(), PlatformError> {
    use std::process::Command;

    let status = Command::new("open").arg(path).status()?;
    if !status.success() {
        return Err(PlatformError::Other(format!(
            "open exited with {status} for {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::macro_engine::MacroError;
    use std::path::PathBuf;

    #[test]
    fn launching_a_missing_path_fails() {
        let missing = PathBuf::from("/definitely/not/a/real/application");
        assert!(launch_path(&missing).is_err());
    }

    #[test]
    fn platform_errors_convert_to_macro_errors() {
        let error = to_macro_error(PlatformError::Unsupported("test sink failure"));
        assert!(
            matches!(&error, MacroError::Sink(message) if message.contains("test sink failure"))
        );
    }

    #[test]
    fn media_commands_route_to_transport_keys() {
        assert_eq!(
            media_key(MediaCommand::PlayPause).ok(),
            to_enigo_key(KeyCode::MediaPlayPause)
        );
        assert_eq!(
            media_key(MediaCommand::Next).ok(),
            to_enigo_key(KeyCode::MediaNext)
        );
        assert_eq!(
            media_key(MediaCommand::Previous).ok(),
            to_enigo_key(KeyCode::MediaPrevious)
        );
        assert_eq!(
            media_key(MediaCommand::Stop).ok(),
            to_enigo_key(KeyCode::MediaStop)
        );
    }
}
