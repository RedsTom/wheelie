//! Key capture: the next key combination pressed in the settings window is
//! shown as pending, then stored by the control that asked for it once the
//! user confirms it. Escape or the cross cancels.

use crate::app::RuntimeHandle;
use crate::core::{Control, KeyChord, KeyCode, Modifiers};
use crate::ui::pages::{actions, devices, macros};
use crate::ui::shell::Shared;
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
    shared.borrow_mut().capture_pending = Some(chord);
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
