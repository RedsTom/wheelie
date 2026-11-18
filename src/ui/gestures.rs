//! Knob gesture handling: detector events to actions and overlay selection.
//!
//! Ported from the eframe shell; window operations are absent until the
//! overlay window lands (F3), so `open_overlay` only updates the state.

use crate::app::RuntimeHandle;
use crate::core::{Control, GestureEvent};
use std::time::{Duration, Instant};

use super::shell::ShellState;

/// Routes one detector event.
pub fn handle_gesture(state: &mut ShellState, gesture: GestureEvent) {
    match gesture {
        GestureEvent::DoublePress => open_overlay(state),
        GestureEvent::RotateLeft => {
            if state.overlay.open {
                let count = state.runtime.config().action_sets.len();
                state.overlay.select_previous(count);
            } else {
                execute(&state.runtime, Control::Left);
            }
        }
        GestureEvent::RotateRight => {
            if state.overlay.open {
                let count = state.runtime.config().action_sets.len();
                state.overlay.select_next(count);
            } else {
                execute(&state.runtime, Control::Right);
            }
        }
        GestureEvent::SinglePress => {
            if state.overlay.open {
                confirm_overlay(state);
            } else {
                execute(&state.runtime, Control::Press);
            }
        }
    }
}

/// Opens the wheel selection for the configured action sets.
pub fn open_overlay(state: &mut ShellState) {
    if state.overlay.open {
        return;
    }
    let config = state.runtime.config();
    let count = config.action_sets.len();
    if count == 0 {
        return;
    }
    let index = config
        .active_set
        .as_deref()
        .and_then(|id| config.action_sets.iter().position(|set| set.id == id))
        .unwrap_or(0);
    state.overlay.open_on(index, count);
    state.detector.set_overlay_open(true);
}

/// Closes the wheel without changing the active set.
pub fn close_overlay(state: &mut ShellState) {
    if !state.overlay.open {
        return;
    }
    state.overlay.open = false;
    state.overlay.outcome = None;
    state.detector.set_overlay_open(false);
}

/// Confirms the selected sector.
pub fn confirm_overlay(state: &mut ShellState) {
    let index = state.overlay.selected;
    activate_set(&state.runtime, index);
    close_overlay(state);
}

/// Makes the set at `index` the active one. The wheel never runs actions.
pub fn activate_set(runtime: &RuntimeHandle, index: usize) {
    let Some(id) = runtime
        .config()
        .action_sets
        .get(index)
        .map(|set| set.id.clone())
    else {
        return;
    };
    if let Err(error) = runtime.update_config(|config| config.active_set = Some(id)) {
        runtime.report_error(error.to_string());
    }
}

/// Runs the action bound to `control` in the active set.
pub fn execute(runtime: &RuntimeHandle, control: Control) {
    let config = runtime.config();
    let Some(set) = config.active_set_or_first() else {
        return;
    };
    let action = set.actions.for_control(control).clone();
    if let Err(error) = runtime.execute(action) {
        runtime.report_error(error.to_string());
    }
}

/// Advances the detector (double-press expiry) and reports how long until
/// the next deadline, so the tick loop can sleep precisely.
pub fn tick_detector(state: &mut ShellState) -> Option<Duration> {
    let now = Instant::now();
    for gesture in state.detector.tick(now) {
        handle_gesture(state, gesture);
    }
    state.detector.pending_expiry().map(|expiry| {
        expiry
            .saturating_duration_since(now)
            .max(Duration::from_millis(5))
    })
}
