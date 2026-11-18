//! OS service wiring for the gpui shell.
//!
//! Ported from the eframe shell: tray and hotkey creation/polling run on the
//! main thread inside the tick, exactly like the old per-frame `update()`.

use crate::app::action_worker::discard_injected;
use crate::app::{AppCommand, AppEvent, RuntimeState};
use crate::core::{Control, InputEvent, TextKey, tr};
use crate::platform::{GlobalHotkeyService, HotkeySource, TrayCommand, TrayService};
use std::time::Instant;
use tracing::{info, warn};

use super::gestures;
use super::shell::ShellState;

/// Drains runtime events into the shell snapshot.
pub fn drain_events(state: &mut ShellState) {
    while let Some(event) = state.runtime.try_recv_event() {
        match event {
            AppEvent::StateChanged(snapshot) => state.snapshot = snapshot,
            AppEvent::Error(message) => state.snapshot.last_error = Some(message),
        }
    }
}

/// Creates the OS services that need the window event loop.
pub fn ensure_services(state: &mut ShellState) {
    if state.hotkeys.is_none() && !state.hotkeys_failed {
        match GlobalHotkeyService::new() {
            Ok(service) => {
                info!("global key watcher ready");
                state.hotkeys = Some(service);
                state.hotkeys_dirty = true;
            }
            Err(error) => {
                warn!(%error, "global key watcher unavailable");
                state.hotkeys_failed = true;
                state.runtime.report_error(error.to_string());
            }
        }
    }

    if state.tray.is_none() && !state.tray_failed && state.runtime.config().general.show_tray {
        let language = state.preferences.language;
        let built =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| TrayService::new(language)));
        match built {
            Ok(Ok(tray)) => {
                state.tray = Some(tray);
                state.tray_language = language;
            }
            Ok(Err(error)) => {
                warn!(%error, "tray icon unavailable");
                state.tray_failed = true;
            }
            Err(_) => {
                warn!("tray icon unavailable on this system");
                state.tray_failed = true;
            }
        }
    }

    if state.tray_language != state.preferences.language
        && let Some(tray) = state.tray.take()
    {
        drop(tray);
        state.tray_failed = false;
    }
}

/// Keeps the gesture detector aligned with the active profile.
pub fn sync_from_config(state: &mut ShellState) {
    let config = state.runtime.config();

    if let Some(profile) = config.active_profile()
        && profile.bindings != state.detector.bindings()
    {
        state.detector.set_bindings(profile.bindings);
        state.hotkeys_dirty = true;
    }

    let gesture_config = config.general.gesture_config();
    if gesture_config != state.gesture_config {
        let bindings = state.detector.bindings();
        let overlay_open = state.overlay.open;
        state.detector = crate::core::GestureDetector::new(bindings, gesture_config);
        state.detector.set_overlay_open(overlay_open);
        state.gesture_config = gesture_config;
    }

    let paused = matches!(
        state.snapshot.state,
        RuntimeState::Paused | RuntimeState::Stopped
    );
    if paused != state.paused {
        state.paused = paused;
        state.hotkeys_dirty = true;
    }
}

/// Re-registers the watched keys after bindings, pause or test changes.
pub fn apply_hotkeys_if_needed(state: &mut ShellState) {
    if !state.hotkeys_dirty {
        return;
    }
    state.hotkeys_dirty = false;

    let Some(service) = &mut state.hotkeys else {
        return;
    };

    let result = if state.paused {
        service.unregister_all()
    } else if let Some(control) = state.testing {
        service.apply(&[state.detector.bindings().chord_for(control)])
    } else {
        let bindings = state.detector.bindings();
        service.apply(&[bindings.left, bindings.right, bindings.press])
    };

    match result {
        Ok(()) => state
            .runtime
            .set_capture_active(!state.paused && state.testing.is_none()),
        Err(error) => {
            state.runtime.set_capture_active(false);
            state.runtime.report_error(error.to_string());
        }
    }
}

/// Drains hotkey events into the gesture detector (or the key tester).
pub fn poll_hotkeys(state: &mut ShellState) {
    let events = match &mut state.hotkeys {
        Some(service) => service.poll(),
        None => Vec::new(),
    };
    let events = discard_injected(events, &state.injection);

    if events.is_empty() {
        return;
    }

    if state.testing.is_some() {
        for event in &events {
            if let InputEvent::Pressed(chord) = event {
                state.last_key = Some(*chord);
            }
        }
        return;
    }

    if state.paused {
        return;
    }

    let now = Instant::now();
    for event in events {
        for gesture in state.detector.feed(event, now) {
            gestures::handle_gesture(state, gesture);
        }
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

/// Drains tray menu commands; window actions are applied by the caller.
pub fn poll_tray(state: &mut ShellState) -> Vec<TrayCommand> {
    state
        .tray
        .as_ref()
        .map(TrayService::poll)
        .unwrap_or_default()
}

/// Name of the active (or first) action set for the sidebar footer.
pub fn active_set_name(state: &ShellState) -> String {
    let config = state.runtime.config();
    let language = state.preferences.language;
    match config.active_set_or_first() {
        Some(set) => set.name.clone(),
        None => tr(language, TextKey::NoSetLabel).to_owned(),
    }
}

/// Sends the pause/resume/quit command matching a tray command.
pub fn apply_tray_command(state: &mut ShellState, command: &TrayCommand) {
    match command {
        TrayCommand::Pause => {
            state.testing = None;
            let _ = state.runtime.send(AppCommand::Pause);
        }
        TrayCommand::Resume => {
            let _ = state.runtime.send(AppCommand::Resume);
        }
        TrayCommand::Quit => {
            state.quitting = true;
            let _ = state.runtime.send(AppCommand::Quit);
        }
        TrayCommand::Show | TrayCommand::Hide => {}
    }
}

/// True when the runtime is not accepting knob input.
pub fn is_paused(state: &ShellState) -> bool {
    matches!(
        state.snapshot.state,
        RuntimeState::Paused | RuntimeState::Stopped
    )
}

/// Guard used by callers that need the active test control.
pub fn testing_control(state: &ShellState) -> Option<Control> {
    state.testing
}
