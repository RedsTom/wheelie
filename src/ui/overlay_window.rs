//! Radial wheel overlay as its own transparent gpui window.
//!
//! The overlay is a 380×380 undecorated `PopUp` centered on the primary
//! display with `WindowBackgroundAppearance::Transparent`. Input updates
//! [`OverlayState`] through the shared shell state; the 35 ms tick calls
//! [`sync`] to process outcomes, watch focus and open/close the window,
//! hiding the settings window while the wheel is up (old morph behavior).

use crate::core::APP_ID;
use crate::ui::gestures;
use crate::ui::overlay::{OVERLAY_SIZE, OverlayOutcome};
use crate::ui::shell::Shared;
use crate::ui::{wheel_paint, window as shell_window};
use gpui::prelude::*;
use gpui::{
    App, Context, FocusHandle, KeyDownEvent, MouseButton, Render, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, canvas, div, point, px,
    size,
};

/// Opens the overlay window unless it is already open.
pub(crate) fn open(shared: &Shared, cx: &mut App) {
    if shared.borrow().overlay_window.is_some() {
        return;
    }

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds::centered(
            None,
            size(px(OVERLAY_SIZE), px(OVERLAY_SIZE)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Wheelie".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some(APP_ID.to_owned()),
        ..Default::default()
    };

    let view_shared = shared.clone();
    let handle = cx
        .open_window(options, move |window, cx| {
            let focus = cx.focus_handle();
            focus.focus(window, cx);
            let build_shared = view_shared.clone();
            cx.new(move |_cx: &mut Context<OverlayView>| OverlayView {
                shared: build_shared,
                focus: focus.clone(),
            })
        })
        .expect("open overlay window");

    shared.borrow_mut().overlay_window = Some(handle);
}

/// Closes the overlay window if open.
pub(crate) fn close(shared: &Shared, cx: &mut App) {
    let handle = shared.borrow_mut().overlay_window.take();
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Processes outcomes, polls focus and keeps the window in sync with
/// `OverlayState`. Called once per tick from the shell.
pub(crate) fn sync(cx: &mut App, shared: &Shared) {
    // Focus-loss close: once the overlay had focus, losing it closes the
    // wheel (old `viewport().focused` polling in the eframe shell).
    let focus_state = {
        let state = shared.borrow();
        (state.overlay_window, state.overlay.focused_once)
    };
    if let Some(handle) = focus_state.0 {
        match handle.is_active(cx) {
            Some(true) => shared.borrow_mut().overlay.focused_once = true,
            Some(false) if focus_state.1 => {
                let mut state = shared.borrow_mut();
                gestures::close_overlay(&mut state);
            }
            _ => {}
        }
    }

    // Consume the outcome produced by input handlers (or knob confirms).
    let outcome = shared.borrow_mut().overlay.outcome.take();
    match outcome {
        Some(OverlayOutcome::Close) => {
            let mut state = shared.borrow_mut();
            gestures::close_overlay(&mut state);
        }
        Some(OverlayOutcome::Confirm(index)) => {
            let mut state = shared.borrow_mut();
            gestures::activate_set(&state.runtime, index);
            gestures::close_overlay(&mut state);
        }
        None => {}
    }

    // Window lifecycle: open with the state, close with it; hide the
    // settings window while the wheel is up and bring it back afterwards.
    let (open, has_window, restore_settings) = {
        let state = shared.borrow();
        (
            state.overlay.open,
            state.overlay_window.is_some(),
            state.settings_hidden_for_overlay,
        )
    };
    if open && !has_window {
        open_window_and_hide_settings(shared, cx);
    } else if !open && has_window {
        close(shared, cx);
        if restore_settings {
            shell_window::focus_settings(cx, shared);
            shared.borrow_mut().settings_hidden_for_overlay = false;
        }
    }
}

fn open_window_and_hide_settings(shared: &Shared, cx: &mut App) {
    shell_window::close_settings(cx, shared);
    shared.borrow_mut().settings_hidden_for_overlay = true;
    open(shared, cx);
}

/// Root view of the wheel window.
pub struct OverlayView {
    /// Shared shell state (selection, outcomes, action sets).
    pub shared: Shared,
    /// Focus handle used for Esc handling.
    pub focus: FocusHandle,
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sets = self.shared.borrow().runtime.config().action_sets.clone();
        let selected = self.shared.borrow().overlay.selected;
        let hovered = self.shared.borrow().overlay.hovered;

        let center = point(px(OVERLAY_SIZE / 2.0), px(OVERLAY_SIZE / 2.0));
        let geometry = wheel_paint::geometry(OVERLAY_SIZE);
        let origin = wheel_paint::origin(center, &geometry);

        // Icons + title over the painted wheel (shared with the settings preview).
        let title_y = f32::from(center.y) + geometry.outer_radius + 22.0;
        let wheel_children =
            wheel_paint::children(&geometry, origin, title_y, OVERLAY_SIZE, &sets, selected);

        // Input state: shared + weak handle to trigger repaints.
        let key_shared = self.shared.clone();
        let key_weak = cx.weak_entity();
        let move_shared = self.shared.clone();
        let move_weak = cx.weak_entity();
        let down_shared = self.shared.clone();
        let down_weak = cx.weak_entity();
        let focus = self.focus.clone();

        div()
            .id("overlay-root")
            .size_full()
            .relative()
            .track_focus(&focus)
            .child(
                canvas(
                    |_, _, _| (),
                    move |_bounds, _, window, _| {
                        wheel_paint::paint(window, &geometry, center, &sets, selected, hovered);
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(wheel_children)
            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    key_shared.borrow_mut().overlay.outcome = Some(OverlayOutcome::Close);
                    if let Some(view) = key_weak.upgrade() {
                        view.update(cx, |_, cx| cx.notify());
                    }
                }
            })
            .on_mouse_move(move |event, _, cx| {
                let count = move_shared.borrow().runtime.config().action_sets.len();
                let hovered = wheel_paint::sector_at(&geometry, count, origin, event.position);
                let mut state = move_shared.borrow_mut();
                if state.overlay.hovered != hovered {
                    state.overlay.hovered = hovered;
                    if let Some(view) = move_weak.upgrade() {
                        view.update(cx, |_, cx| cx.notify());
                    }
                }
            })
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                let count = down_shared.borrow().runtime.config().action_sets.len();
                let outcome = match wheel_paint::sector_at(&geometry, count, origin, event.position)
                {
                    Some(index) => OverlayOutcome::Confirm(index),
                    None if wheel_paint::is_center(&geometry, origin, event.position) => {
                        let selected = down_shared.borrow().overlay.selected;
                        OverlayOutcome::Confirm(selected)
                    }
                    None => OverlayOutcome::Close,
                };
                down_shared.borrow_mut().overlay.outcome = Some(outcome);
                if let Some(view) = down_weak.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            })
    }
}
