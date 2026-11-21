//! Radial wheel overlay as its own transparent gpui window.
//!
//! The overlay is a 380×380 undecorated `PopUp` centered on the primary
//! display with `WindowBackgroundAppearance::Transparent`. Mouse and keyboard
//! input update [`OverlayState`] through the shared shell state; the 35 ms tick
//! calls [`sync`] to process outcomes, watch focus and open/close the window.
//! The wheel is always its own window; the settings window stays open.

use crate::core::APP_ID;
use crate::platform::window_frame;
use crate::ui::gestures;
use crate::ui::shell::{OverlayClosing, Shared, WINDOW_OPEN_RETRY};
use crate::ui::wheel::{OVERLAY_SIZE, OverlayOutcome};
use crate::ui::wheel_paint;
use gpui::prelude::*;
use gpui::{
    App, ClickEvent, Context, FocusHandle, KeyDownEvent, MouseMoveEvent, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, canvas,
    div, point, px, size,
};
use std::time::{Duration, Instant};

/// Time given to gpui to deliver the deactivation callback after the overlay
/// is hidden. The callback is dispatched asynchronously and would otherwise
/// try to update a window gpui has already removed.
const CLOSE_DELAY: Duration = Duration::from_millis(120);

/// Opens the overlay window unless it is already open or its last open failed
/// recently. A refused window is logged and retried on a later tick.
pub(crate) fn open(shared: &Shared, cx: &mut App) {
    let state = shared.borrow();
    if state.overlay_window.is_some()
        || state
            .overlay_open_retry_at
            .is_some_and(|at| at.elapsed() < WINDOW_OPEN_RETRY)
    {
        return;
    }
    drop(state);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds::centered(
            None,
            size(px(OVERLAY_SIZE), px(OVERLAY_SIZE)),
            cx,
        ))),
        titlebar: None,
        window_decorations: Some(WindowDecorations::Client),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some(APP_ID.to_owned()),
        show: false,
        ..Default::default()
    };

    let view_shared = shared.clone();
    match cx.open_window(options, move |window, cx| {
        // Created hidden: the frame is removed before the first frame is
        // shown, so the border never flashes.
        if let Err(error) = window_frame::strip_frame(&*window) {
            tracing::warn!(%error, "could not remove the wheel window frame");
        }
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let build_shared = view_shared.clone();
        cx.new(move |_cx: &mut Context<OverlayView>| OverlayView {
            shared: build_shared,
            focus: focus.clone(),
        })
    }) {
        Ok(handle) => {
            // Showing a window created hidden goes through `activate_window`.
            let _ = handle.update(cx, |_, window, _| window.activate_window());
            let mut state = shared.borrow_mut();
            state.overlay_window = Some(handle);
            state.overlay_open_retry_at = None;
        }
        Err(error) => {
            tracing::warn!(%error, "wheel window unavailable; retrying later");
            shared.borrow_mut().overlay_open_retry_at = Some(Instant::now());
        }
    }
}

/// Starts closing the overlay window.
///
/// The window is hidden immediately so Windows delivers `WM_ACTIVATE` while
/// gpui still knows the window. gpui dispatches that callback asynchronously,
/// so removal is deferred until [`sync`] confirms the delay elapsed. Removing
/// an active window straight away makes the late callback log "window not
/// found".
pub(crate) fn close(shared: &Shared, cx: &mut App) {
    let handle = shared.borrow_mut().overlay_window.take();
    shared.borrow_mut().overlay_open_retry_at = None;
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window_frame::hide_window(window));
        let _ = handle.update(cx, |_, window, cx| window.blur(cx));
        shared.borrow_mut().overlay_closing = Some(OverlayClosing {
            handle,
            remove_at: Instant::now() + CLOSE_DELAY,
        });
    }
}

/// Processes outcomes, polls focus and keeps the window in sync with
/// `OverlayState`. Called once per tick from the shell.
pub(crate) fn sync(cx: &mut App, shared: &Shared) {
    // Finish a close only after gpui has drained the deactivation callback.
    let closing_ready = shared
        .borrow()
        .overlay_closing
        .as_ref()
        .is_some_and(|closing| Instant::now() >= closing.remove_at);
    if closing_ready && let Some(closing) = shared.borrow_mut().overlay_closing.take() {
        let _ = closing
            .handle
            .update(cx, |_, window, _| window.remove_window());
    }

    // Focus-loss close: once the overlay had focus, losing it closes the wheel.
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
            gestures::select_set(&state.runtime, index);
            gestures::close_overlay(&mut state);
        }
        None => {}
    }

    // Window lifecycle: the wheel is always its own window; the settings
    // window stays open behind it. A pending close must finish before a new
    // window opens, otherwise both handles point at the same native window.
    let (should_open, has_window, closing) = {
        let state = shared.borrow();
        (
            state.overlay.open,
            state.overlay_window.is_some(),
            state.overlay_closing.is_some(),
        )
    };
    if should_open && !has_window && !closing {
        open(shared, cx);
    } else if !should_open && has_window {
        close(shared, cx);
    }

    // Knob rotation changes the selection without any pointer event, so the
    // wheel window must be repainted every tick while it is open.
    if should_open {
        notify(shared, cx);
    }
}

/// Repaints the wheel window so knob-driven selection shows up immediately.
fn notify(shared: &Shared, cx: &mut App) {
    let handle = shared.borrow().overlay_window;
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, _, cx| cx.notify());
    }
}

/// Root view of the wheel window.
pub struct OverlayView {
    /// Shared shell state (selection, outcomes, action sets).
    pub shared: Shared,
    /// Focus handle used for keyboard navigation and Esc handling.
    pub focus: FocusHandle,
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (sets, selected, hovered) = {
            let state = self.shared.borrow();
            (
                state.runtime.config().action_sets.clone(),
                state.overlay.selected,
                state.overlay.hovered,
            )
        };

        let center = point(px(OVERLAY_SIZE / 2.0), px(OVERLAY_SIZE / 2.0));
        let geometry = wheel_paint::geometry(OVERLAY_SIZE);
        let origin = wheel_paint::origin(center, &geometry);

        // Icons + title over the painted wheel (shared with the settings preview).
        let title_y = wheel_paint::title_y(center, &geometry);
        let wheel_children = wheel_paint::children(
            &geometry,
            origin,
            title_y,
            OVERLAY_SIZE,
            &sets,
            selected,
            true,
        );

        // Input state: shared + weak handle to trigger repaints.
        let key_shared = self.shared.clone();
        let key_weak = cx.weak_entity();
        let mouse_shared = self.shared.clone();
        let mouse_weak = cx.weak_entity();
        let click_shared = self.shared.clone();
        let click_weak = cx.weak_entity();
        let focus = self.focus.clone();

        div()
            .id("overlay-root")
            .size_full()
            .relative()
            .track_focus(&focus)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let paint_center = point(
                            bounds.origin.x + px(OVERLAY_SIZE / 2.0),
                            bounds.origin.y + px(OVERLAY_SIZE / 2.0),
                        );
                        wheel_paint::paint(
                            window,
                            &geometry,
                            paint_center,
                            &sets,
                            selected,
                            hovered,
                        );
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(wheel_children)
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let count = mouse_shared.borrow().runtime.config().action_sets.len();
                let x = f32::from(event.position.x) - f32::from(origin.x);
                let y = f32::from(event.position.y) - f32::from(origin.y);
                let sector = geometry.sector_at(count, x, y);
                let changed = {
                    let mut state = mouse_shared.borrow_mut();
                    if state.overlay.hovered == sector {
                        false
                    } else {
                        state.overlay.hovered = sector;
                        true
                    }
                };
                if changed && let Some(view) = mouse_weak.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            })
            .on_click(move |event: &ClickEvent, _, cx| {
                cx.stop_propagation();
                let count = click_shared.borrow().runtime.config().action_sets.len();
                let x = f32::from(event.position().x) - f32::from(origin.x);
                let y = f32::from(event.position().y) - f32::from(origin.y);
                if let Some(index) = geometry.sector_at(count, x, y) {
                    let mut state = click_shared.borrow_mut();
                    state.overlay.selected = index;
                    state.overlay.hovered = Some(index);
                    state.overlay.outcome = Some(OverlayOutcome::Confirm(index));
                }
                if let Some(view) = click_weak.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            })
            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                let key = event.keystroke.key.as_str();
                let handled = matches!(key, "escape" | "enter" | "tab" | "left" | "right");
                if !handled {
                    return;
                }
                cx.stop_propagation();
                let mut state = key_shared.borrow_mut();
                match key {
                    "escape" => state.overlay.outcome = Some(OverlayOutcome::Close),
                    "enter" => {
                        let index = state.overlay.selected;
                        state.overlay.outcome = Some(OverlayOutcome::Confirm(index));
                    }
                    "left" | "tab" => {
                        let count = state.runtime.config().action_sets.len();
                        state.overlay.select_previous(count);
                    }
                    "right" => {
                        let count = state.runtime.config().action_sets.len();
                        state.overlay.select_next(count);
                    }
                    _ => {}
                }
                drop(state);
                if let Some(view) = key_weak.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            })
    }
}
