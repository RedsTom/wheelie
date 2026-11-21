//! Event routing for the gpui shell.
//!
//! OS services (global keys and tray) are owned by the application layer and
//! created/polled on the thread that runs the window event loop; see the
//! documented exception on [`crate::app::RuntimeHandle::tick_services`]. This
//! module only routes the resulting events into UI state and reports window
//! changes back to the runtime. It makes no OS calls and takes no runtime
//! decisions.

use crate::app::{AppEvent, RuntimeState, TrayCommand};
use crate::core::{GestureDetector, InputEvent};
use crate::ui::osd_window::Osd;
use std::time::Instant;

use super::shell::ShellState;
use super::{capture, gestures};

/// Drains runtime events into the shell snapshot and the gesture detector.
///
/// Returns tray commands that only the shell may apply: windows and process
/// exit. Runtime-state commands are handled here and not returned.
pub fn drain_events(state: &mut ShellState) -> Vec<TrayCommand> {
    let mut window_commands = Vec::new();
    while let Some(event) = state.runtime.try_recv_event() {
        match event {
            AppEvent::StateChanged(snapshot) => state.snapshot = snapshot,
            AppEvent::Error(message) => state.snapshot.last_error = Some(message),
            AppEvent::Indicator(indicator) => {
                state.osd = Some(Osd::new(indicator));
            }
            AppEvent::Hotkeys(events) => route_hotkeys(state, events),
            AppEvent::Tray(commands) => {
                for command in commands {
                    if matches!(
                        command,
                        TrayCommand::Show | TrayCommand::Hide | TrayCommand::Quit
                    ) {
                        window_commands.push(command);
                    }
                    apply_tray_command(state, &command);
                }
            }
        }
    }
    window_commands
}

/// Sends tray commands that change the runtime state; window commands are
/// consumed by the caller because only the shell can open or close windows.
pub fn apply_tray_command(state: &mut ShellState, command: &TrayCommand) {
    match command {
        TrayCommand::Pause => {
            let _ = state.runtime.send(crate::app::AppCommand::Pause);
        }
        TrayCommand::Resume => {
            let _ = state.runtime.send(crate::app::AppCommand::Resume);
        }
        TrayCommand::Quit => {
            state.quitting = true;
            let _ = state.runtime.send(crate::app::AppCommand::Quit);
        }
        TrayCommand::Show => state.settings_wanted = true,
        TrayCommand::Hide => state.settings_wanted = false,
    }
}

/// Routes hotkey events to the pending key field or the gesture detector.
fn route_hotkeys(state: &mut ShellState, events: Vec<InputEvent>) {
    if events.is_empty() {
        return;
    }
    if state.capture.is_some() {
        for event in events {
            if let InputEvent::Pressed(chord) = event {
                capture::record_chord(state, chord);
            }
        }
        return;
    }
    if matches!(
        state.snapshot.state,
        RuntimeState::Paused | RuntimeState::Stopped
    ) {
        return;
    }

    let now = Instant::now();
    for event in events {
        for gesture in state.detector.feed(event, now) {
            gestures::handle_gesture(state, gesture);
        }
    }
}

/// Polls OS services once and publishes the window state they need.
///
/// This is the only per-tick call that reaches the OS, and it stays inside the
/// application layer so the UI never names a platform adapter.
pub fn tick(state: &mut ShellState) {
    state.runtime.tick_services();
    state.snapshot.healthy = state.runtime.health();
    state
        .runtime
        .set_window_visible(state.settings_window.is_some());
    state
        .runtime
        .set_key_capture_active(state.capture.is_some());
}

/// Keeps the gesture detector aligned with the active profile and validates
/// the page's ephemeral selections against the current configuration.
pub fn sync_from_config(state: &mut ShellState) {
    let config = state.runtime.config();

    if let Some(profile) = config.active_profile()
        && profile.bindings != state.detector.bindings()
    {
        state.detector.set_bindings(profile.bindings);
    }

    let gesture_config = config.general.gesture_config();
    if gesture_config != state.gesture_config {
        let bindings = state.detector.bindings();
        let overlay_open = state.overlay.open;
        state.detector = GestureDetector::new(bindings, gesture_config);
        state.detector.set_overlay_open(overlay_open);
        state.gesture_config = gesture_config;
    }

    validate_selections(state, &config);
}

/// Drops selections and captures that no longer name an existing item.
fn validate_selections(state: &mut ShellState, config: &crate::core::Config) {
    let default_set = || config.action_sets.first().map(|set| set.id.clone());
    let selected_set = state
        .page
        .selected_set
        .take()
        .filter(|id| config.action_sets.iter().any(|set| &set.id == id))
        .or_else(default_set);
    state.page.selected_set = selected_set;

    let default_macro = || config.macros.first().map(|item| item.name.clone());
    let selected_macro = state
        .page
        .selected_macro
        .take()
        .filter(|name| config.macros.iter().any(|item| &item.name == name))
        .or_else(default_macro);
    state.page.selected_macro = selected_macro;

    if let Some(name) = &state.page.selected_macro {
        let steps = config
            .macros
            .iter()
            .find(|item| &item.name == name)
            .map(|item| item.steps.len())
            .unwrap_or(0);
        if state.page.selected_step.is_some_and(|index| index >= steps) {
            state.page.selected_step = None;
            state.page.open_menu = None;
        }
    } else {
        state.page.selected_step = None;
    }

    if state.page.open_menu.is_some() && state.page.selected_set.is_none() {
        state.page.open_menu = None;
    }
}

/// Persists language and theme changes back to the configuration.
pub fn persist_preferences(state: &mut ShellState) {
    let config = state.runtime.config();
    if config.language == state.preferences.language
        && config.general.theme == state.preferences.theme_mode
    {
        return;
    }

    let language = state.preferences.language;
    let theme_mode = state.preferences.theme_mode;
    if let Err(error) = state.runtime.update_config(|config| {
        config.language = language;
        config.general.theme = theme_mode;
    }) {
        state.runtime.report_error(error.to_string());
    }
}
