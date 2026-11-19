//! Key capture: the next key combination pressed in the settings window is
//! shown as pending, then stored by the control that asked for it once the
//! user confirms it. Escape or the cross cancels.

use crate::app::RuntimeHandle;
use crate::core::{Control, KeyChord, KeyCode, Modifiers};
use crate::ui::pages::{actions, devices, macros};
use crate::ui::shell::{Shared, ShellState};
use gpui::{App, KeyDownEvent, Window};
use std::str::FromStr;

/// Value waiting for a captured key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTarget {
    /// Key of a knob control on the devices page.
    Binding(Control),
    /// Shortcut of one control in an action set.
    Shortcut { set_id: String, control: Control },
    /// Key of a key step in a macro.
    MacroStep { name: String, index: usize },
}

/// Starts waiting for a key combination for `target`. The settings root takes
/// focus so the key events reach [`handle_key`].
pub fn start(window: &mut Window, cx: &mut App, shared: &Shared, target: CaptureTarget) {
    let focus = {
        let mut state = shared.borrow_mut();
        state.capture = Some(target);
        state.capture_pending = None;
        state.page.focus_capture.clone()
    };
    focus.focus(window, cx);
}

/// Chords a field can only record through the global key watcher.
///
/// Windows exposes volume and media keys as virtual keys that gpui does not
/// translate into `KeyDownEvent`, and the desktop shell answers its own
/// shortcuts (Alt+Tab, the launcher, the Windows chords) before the focused
/// window ever sees them. While a field waits, those chords are taken over
/// globally so the press reaches the field instead of the system.
pub fn capture_only_chords() -> Vec<KeyChord> {
    KeyChord::system_reserved()
}

/// Records a chord delivered by the global key watcher while a field waits.
/// Returns true when the chord was used.
pub fn record_chord(state: &mut ShellState, chord: KeyChord) -> bool {
    if state.capture.is_none() {
        return false;
    }
    state.capture_pending = Some(chord);
    true
}

/// Whether `target` is the capture in progress, and the text of its key chip:
/// the pending combination, the hint while waiting, or the stored `current`.
pub fn chip(
    state: &crate::ui::shell::ShellState,
    target: &CaptureTarget,
    current: KeyChord,
    hint: &str,
) -> (bool, String) {
    if state.capture.as_ref() != Some(target) {
        return (false, current.to_string());
    }
    let label = match state.capture_pending {
        Some(pending) => pending.to_string(),
        None => hint.to_owned(),
    };
    (true, label)
}

/// Records the combination pressed while a capture is pending. Returns true
/// when the event was used, so the settings view can refresh.
pub fn handle_key(shared: &Shared, event: &KeyDownEvent) -> bool {
    if shared.borrow().capture.is_none() {
        return false;
    }

    if event.keystroke.key.eq_ignore_ascii_case("escape") {
        cancel(shared);
        return true;
    }

    // Modifier-only or unmapped keys leave the pending combination unchanged.
    let Ok(key) = KeyCode::from_str(&event.keystroke.key) else {
        return true;
    };
    let held = event.keystroke.modifiers;
    let chord = KeyChord::with_modifiers(
        key,
        Modifiers {
            control: held.control,
            alt: held.alt,
            shift: held.shift,
            meta: held.platform,
        },
    );
    record_chord(&mut shared.borrow_mut(), chord);
    true
}

/// Stores the pending combination in the control that asked for it.
pub fn confirm(shared: &Shared) {
    let (target, chord, runtime) = {
        let mut state = shared.borrow_mut();
        let Some(chord) = state.capture_pending.take() else {
            return;
        };
        let Some(target) = state.capture.take() else {
            return;
        };
        (target, chord, state.runtime.clone())
    };
    apply(shared, &runtime, target, chord);
}

/// Drops the pending combination and leaves the capture.
pub fn cancel(shared: &Shared) {
    let mut state = shared.borrow_mut();
    state.capture = None;
    state.capture_pending = None;
}

fn apply(shared: &Shared, runtime: &RuntimeHandle, target: CaptureTarget, chord: KeyChord) {
    match target {
        CaptureTarget::Binding(control) => devices::capture_chord(shared, control, chord),
        CaptureTarget::Shortcut { set_id, control } => {
            actions::capture_chord(runtime, &set_id, control, chord);
        }
        CaptureTarget::MacroStep { name, index } => {
            macros::capture_chord(runtime, &name, index, chord);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_only_chords_cover_system_keys() {
        let chords = capture_only_chords();
        for key in [
            KeyCode::VolumeUp,
            KeyCode::VolumeDown,
            KeyCode::VolumeMute,
            KeyCode::MediaPlayPause,
            KeyCode::MediaNext,
            KeyCode::MediaPrevious,
            KeyCode::MediaStop,
        ] {
            assert!(
                chords.contains(&KeyChord::new(key)),
                "missing capture chord for {key}"
            );
        }
    }

    #[test]
    fn capture_only_chords_take_over_shell_shortcuts() {
        use crate::core::key::Modifiers;

        let with = |key, modifiers: Modifiers| KeyChord::with_modifiers(key, modifiers);
        let chords = capture_only_chords();

        assert!(
            chords.contains(&with(
                KeyCode::Tab,
                Modifiers {
                    alt: true,
                    ..Default::default()
                }
            )),
            "Alt+Tab must be recorded, not switch windows"
        );
        assert!(
            chords.contains(&with(
                KeyCode::KeyD,
                Modifiers {
                    meta: true,
                    ..Default::default()
                }
            )),
            "the show-desktop chord must be recorded"
        );
    }

    #[test]
    fn capture_only_chords_leave_regular_keys_to_the_window() {
        let chords = capture_only_chords();
        assert!(!chords.contains(&KeyChord::new(KeyCode::F13)));
        assert!(!chords.contains(&KeyChord::new(KeyCode::KeyA)));
        assert!(
            !chords.contains(&KeyChord::new(KeyCode::Escape)),
            "Escape must always cancel the field"
        );
    }
}
