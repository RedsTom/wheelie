//! Radial wheel overlay as its own transparent gpui window.
//!
//! The overlay is a 380×380 undecorated `PopUp` centered on the primary
//! display with `WindowBackgroundAppearance::Transparent`. Input updates
//! [`OverlayState`] through the shared shell state; the 35 ms tick calls
//! [`sync`] to process outcomes, watch focus and open/close the window.
//! The wheel is always its own window; the settings window stays open.

use crate::core::APP_ID;
use crate::platform::window_frame;
use crate::ui::gestures;
use crate::ui::shell::{OverlayClosing, Shared};
use crate::ui::wheel::{OVERLAY_SIZE, OverlayOutcome};
use crate::ui::wheel_paint;
use gpui::prelude::*;
use gpui::{
    App, Context, FocusHandle, KeyDownEvent, Render, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, canvas, div, point, px, size,
};
use std::time::{Duration, Instant};

/// Time given to gpui to deliver the deactivation callback after the overlay
/// is hidden. The callback is dispatched asynchronously and would otherwise
/// try to update a window gpui has already removed.
const CLOSE_DELAY: Duration = Duration::from_millis(120);

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
    let handle = cx
        .open_window(options, move |window, cx| {
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
        })
        .expect("open overlay window");

    // Showing a window created hidden goes through `activate_window`.
    let _ = handle.update(cx, |_, window, _| window.activate_window());
    shared.borrow_mut().overlay_window = Some(handle);
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
            gestures::activate_set(&state.runtime, index);
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
    /// Focus handle used for Esc handling.
    pub focus: FocusHandle,
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sets = self.shared.borrow().runtime.config().action_sets.clone();
        let selected = self.shared.borrow().overlay.selected;
        // The wheel ignores the mouse: it is driven by the knob and Escape.
        let hovered = None;

        let center = point(px(OVERLAY_SIZE / 2.0), px(OVERLAY_SIZE / 2.0));
        let geometry = wheel_paint::geometry(OVERLAY_SIZE);
        let origin = wheel_paint::origin(center, &geometry);

        // Icons + title over the painted wheel (shared with the settings preview).
        let title_y = f32::from(center.y) + geometry.outer_radius + 22.0;
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
            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    key_shared.borrow_mut().overlay.outcome = Some(OverlayOutcome::Close);
                    if let Some(view) = key_weak.upgrade() {
                        view.update(cx, |_, cx| cx.notify());
                    }
                }
            })
    }
}
