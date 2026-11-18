//! Transient volume indicator shown when the system volume changes.
//!
//! Volume changes made through the audio API do not trigger the system
//! indicator, so Wheelie draws its own. The window is a borderless,
//! transparent popup near the bottom of the primary display that never takes
//! focus and closes itself after [`DISPLAY_DURATION`].

use crate::app::Indicator;
use crate::core::{APP_ID, SetIcon};
use crate::platform::window_frame;
use crate::ui::shell::Shared;
use crate::ui::{iconset, style};
use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, Render, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowHandle, WindowKind, WindowOptions, div, point, px, relative, size,
};
use std::time::{Duration, Instant};

const WIDTH: f32 = 260.0;
const HEIGHT: f32 = 56.0;
const BOTTOM_MARGIN: f32 = 64.0;
const ICON_SIZE: f32 = 18.0;

/// How long the indicator stays on screen after the last volume change.
pub const DISPLAY_DURATION: Duration = Duration::from_millis(1500);

/// Level shown by the indicator and the moment it disappears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Osd {
    /// Level reported right after the change.
    pub indicator: Indicator,
    /// Instant after which the indicator closes.
    pub until: Instant,
}

impl Osd {
    /// Indicator for `indicator`, visible for [`DISPLAY_DURATION`] from now.
    pub fn new(indicator: Indicator) -> Self {
        Self {
            indicator,
            until: Instant::now() + DISPLAY_DURATION,
        }
    }
}

/// Bounds of the indicator: horizontally centered, near the bottom edge of
/// the primary display. Falls back to a centered window without a display.
fn bounds(cx: &App) -> Bounds<gpui::Pixels> {
    let window_size = size(px(WIDTH), px(HEIGHT));
    match cx.primary_display().map(|display| display.bounds()) {
        Some(screen) => Bounds {
            origin: point(
                screen.center().x - px(WIDTH / 2.0),
                screen.bottom() - px(HEIGHT + BOTTOM_MARGIN),
            ),
            size: window_size,
        },
        None => Bounds::centered(None, window_size, cx),
    }
}

/// Opens the indicator window unless it is already open.
fn open(shared: &Shared, cx: &mut App) {
    if shared.borrow().osd_window.is_some() {
        return;
    }

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds(cx))),
        titlebar: None,
        window_decorations: Some(WindowDecorations::Client),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        focus: false,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some(APP_ID.to_owned()),
        ..Default::default()
    };

    let view_shared = shared.clone();
    let handle = cx
        .open_window(options, move |window, cx| {
            if let Err(error) = window_frame::strip_frame(&*window) {
                tracing::warn!(%error, "could not remove the volume indicator frame");
            }
            cx.new(move |_: &mut Context<OsdView>| OsdView {
                shared: view_shared.clone(),
            })
        })
        .expect("open volume indicator window");

    shared.borrow_mut().osd_window = Some(handle);
}

/// Closes the indicator window if open.
fn close(shared: &Shared, cx: &mut App) {
    let handle = shared.borrow_mut().osd_window.take();
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Expires the indicator, then keeps its window in sync with the shell state.
/// Called once per tick from the shell.
pub(crate) fn sync(cx: &mut App, shared: &Shared) {
    let (visible, has_window) = {
        let mut state = shared.borrow_mut();
        if state.osd.is_some_and(|osd| Instant::now() >= osd.until) {
            state.osd = None;
        }
        (state.osd.is_some(), state.osd_window.is_some())
    };

    if visible && !has_window {
        open(shared, cx);
    } else if !visible && has_window {
        close(shared, cx);
    }

    if visible {
        let handle: Option<WindowHandle<OsdView>> = shared.borrow().osd_window;
        if let Some(handle) = handle {
            let _ = handle.update(cx, |_, _, cx| cx.notify());
        }
    }
}

/// Root view of the indicator window.
pub struct OsdView {
    /// Shared shell state (current volume and theme).
    pub shared: Shared,
}

impl Render for OsdView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.shared.borrow();
        let t = style::tokens(state.preferences.theme_mode);
        let (icon, shown) = match state.osd.map(|osd| osd.indicator) {
            Some(Indicator::Volume(level)) => {
                (SetIcon::Volume, if level.muted { 0 } else { level.percent })
            }
            Some(Indicator::Brightness(percent)) => (SetIcon::Brightness, percent),
            None => (SetIcon::Volume, 0),
        };
        let fraction = f32::from(shown) / 100.0;

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .font_family("Rubik")
            .text_size(px(14.0))
            .child(
                div()
                    .size_full()
                    .px(px(16.0))
                    .rounded(px(style::RADIUS_CARD))
                    .bg(t.card)
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(iconset::element(icon, ICON_SIZE, t.text))
                    .child(
                        div().flex_1().h(px(6.0)).rounded_full().bg(t.border).child(
                            div()
                                .h_full()
                                .w(relative(fraction))
                                .rounded_full()
                                .bg(style::ACCENT),
                        ),
                    )
                    .child(
                        div()
                            .w(px(28.0))
                            .flex()
                            .justify_end()
                            .text_color(t.text)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(shown.to_string()),
                    ),
            )
    }
}
