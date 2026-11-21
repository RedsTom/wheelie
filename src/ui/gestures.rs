//! Knob gesture routing: detector events to runtime intents and wheel state.
//!
//! `open_overlay` only updates the wheel state; the overlay window itself is
//! owned by the shell. The runtime owns every business decision: selecting a
//! set and resolving/executing a control are its typed intents.

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
                run_control(&state.runtime, Control::Left);
            }
        }
        GestureEvent::RotateRight => {
            if state.overlay.open {
                let count = state.runtime.config().action_sets.len();
                state.overlay.select_next(count);
            } else {
                run_control(&state.runtime, Control::Right);
            }
        }
        GestureEvent::SinglePress => {
            if state.overlay.open {
                confirm_overlay(state);
            } else {
                run_control(&state.runtime, Control::Press);
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
    let index = active_index(&config);
    state.overlay.open_on(index, count);
    state.detector.set_overlay_open(true);
}

/// Index of the active set, falling back to the first one when the stored id
/// is stale. The wheel only paints sets that exist in the current config.
fn active_index(config: &crate::core::Config) -> usize {
    config
        .active_set
        .as_deref()
        .and_then(|id| config.action_sets.iter().position(|set| set.id == id))
        .unwrap_or(0)
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
    select_set(&state.runtime, index);
    close_overlay(state);
}

/// Makes the set at `index` the active one. The wheel never runs actions.
pub fn select_set(runtime: &RuntimeHandle, index: usize) {
    if let Err(error) = runtime.select_set(index) {
        runtime.report_error(error.to_string());
    }
}

/// Runs the action bound to `control` in the active set.
pub fn run_control(runtime: &RuntimeHandle, control: Control) {
    if let Err(error) = runtime.run_control(control) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Config;

    /// A stale active-set id opens the wheel on the first existing set instead
    /// of showing a selection that cannot be confirmed.
    #[test]
    fn a_stale_active_set_falls_back_to_the_first_set() {
        let mut config = Config {
            active_set: Some("missing-set".to_owned()),
            ..Default::default()
        };
        assert_eq!(active_index(&config), 0);

        config.active_set = Some(config.action_sets[1].id.clone());
        assert_eq!(active_index(&config), 1);
    }

    /// The wheel never opens without a set to select.
    #[test]
    fn an_empty_configuration_leaves_the_wheel_closed() {
        let mut config = Config::default();
        config.action_sets.clear();
        let mut runtime = config.clone();
        runtime.active_set = None;
        assert_eq!(active_index(&runtime), 0);
    }
}
