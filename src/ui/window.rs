//! Settings window lifecycle for the gpui shell.
//!
//! gpui has no hide/show API, so close-to-tray works by removing the window
//! (`QuitMode::Explicit` keeps the app alive) and recreating it on tray Show.

use crate::core::APP_ID;
use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions, px,
    size,
};

use super::shell::{SettingsView, Shared};

/// Initial window size, 16:9.
const DEFAULT_WIDTH: f32 = 1280.0;
const DEFAULT_HEIGHT: f32 = 720.0;

/// Window options for the settings shell.
pub fn settings_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::from("Wheelie")),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(DEFAULT_WIDTH), px(DEFAULT_HEIGHT)),
            cx,
        ))),
        window_min_size: Some(size(px(960.0), px(540.0))),
        app_id: Some(APP_ID.to_owned()),
        ..Default::default()
    }
}

/// Opens the settings window unless it is already open.
pub fn open_settings(cx: &mut App, shared: &Shared) {
    if shared.borrow().settings_window.is_some() {
        return;
    }

    let options = {
        let state = shared.borrow();
        let mut options = settings_window_options(cx);
        // Restore the last size after a close-to-tray recreate.
        if let Some(bounds) = state.settings_bounds {
            options.window_bounds = Some(WindowBounds::Windowed(bounds));
        }
        options
    };

    let close_shared = shared.clone();
    // gpui-component stores per-window state on `gpui_base::Root`. Opening the
    // window with plain `cx.open_window` leaves that state missing and makes
    // `open_alert_dialog` panic when a page opens a confirmation dialog.
    let (handle, view) = gpui_kit::open_window(options, cx, move |window, cx| {
        register_close_behavior(window, cx, &close_shared);
        let view_shared = close_shared.clone();
        cx.new(move |_: &mut Context<SettingsView>| SettingsView {
            shared: view_shared,
        })
    })
    .expect("open settings window");

    let mut state = shared.borrow_mut();
    state.settings_window = Some(handle);
    state.settings_view = Some(view);
}

/// Closes the settings window if open (used when quitting).
pub fn close_settings(cx: &mut App, shared: &Shared) {
    let mut state = shared.borrow_mut();
    let handle = state.settings_window.take();
    state.settings_view = None;
    drop(state);
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Brings the settings window to the front, recreating it when needed.
pub fn focus_settings(cx: &mut App, shared: &Shared) {
    open_settings(cx, shared);
    let handle = shared.borrow().settings_window;
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
        cx.activate(false);
    }
}

/// Clears handles of windows the user closed, remembering the last size.
pub fn reap_closed(cx: &mut App, shared: &Shared) {
    let mut state = shared.borrow_mut();
    if let Some(handle) = state.settings_window {
        match handle.update(cx, |_, window, _| window.window_bounds()) {
            Ok(bounds) => {
                if let WindowBounds::Windowed(bounds) = bounds {
                    state.settings_bounds = Some(bounds);
                }
            }
            Err(_) => {
                state.settings_window = None;
                state.settings_view = None;
            }
        }
    }
}

/// Repaints the settings view so shared-state changes show up.
pub fn notify_settings(cx: &mut App, shared: &Shared) {
    let view = shared.borrow().settings_view.clone();
    if let Some(view) = view {
        view.update(cx, |_, cx| cx.notify());
    }
}

/// Close semantics: hide-to-tray by default; quit the app when the tray is off.
fn register_close_behavior(window: &mut Window, cx: &mut App, shared: &Shared) {
    let close_shared = shared.clone();
    window.on_window_should_close(cx, move |_, cx| {
        let show_tray = close_shared.borrow().runtime.config().general.show_tray;
        let quitting = close_shared.borrow().quitting;
        if !show_tray && !quitting {
            cx.quit();
        }
        // Allowed either way: the window is removed, the app survives when
        // the tray is on, and tray Show recreates the window.
        true
    });
}
