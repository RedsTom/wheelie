//! Transient indicators shown when the volume, brightness or media session
//! changes.
//!
//! Changes made through the audio API do not trigger the system indicator,
//! and media transport keys leave no visible trace of their own, so Wheelie
//! draws both. The window is a borderless, transparent popup near the bottom
//! of the primary display that never takes focus and closes itself after the
//! indicator's display duration.

use crate::app::{CoverImageFormat, CoverView, Indicator, MEDIA_DISPLAY_DURATION, PlayingView};
use crate::core::{APP_ID, SetIcon};
use crate::platform::window_frame;
use crate::ui::shell::{Shared, WINDOW_OPEN_RETRY};
use crate::ui::{iconset, style};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Image, ImageFormat, ImageSource, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, div,
    img, point, px, relative, size,
};
use gpui_icons::LucideIcon;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Width of the volume/brightness bar.
const WIDTH: f32 = 260.0;
/// Height of the volume/brightness bar.
const HEIGHT: f32 = 56.0;
/// Width of the now-playing card.
const MEDIA_WIDTH: f32 = 360.0;
/// Height of the now-playing card (a line taller: the progress row).
const MEDIA_HEIGHT: f32 = 76.0;
/// Top and bottom padding of the now-playing card.
const PADDING_VERTICAL: f32 = 10.0;
const BOTTOM_MARGIN: f32 = 64.0;
const ICON_SIZE: f32 = 18.0;
/// The cover fills the card between the vertical paddings, and the status
/// glyph sits in a square of the same size.
const COVER_SIZE: f32 = MEDIA_HEIGHT - 2.0 * PADDING_VERTICAL;

/// How long a level indicator stays on screen after the last change.
pub const DISPLAY_DURATION: Duration = Duration::from_millis(1500);
/// Indicator shown and the moment it disappears.
#[derive(Debug, Clone, PartialEq)]
pub struct Osd {
    /// The reported level or track.
    pub indicator: Indicator,
    /// Instant after which the indicator closes.
    pub until: Instant,
    /// Instant the indicator was sampled, to advance a playing track.
    pub sampled_at: Instant,
}

impl Osd {
    /// Indicator for `indicator`, visible for its display duration from now.
    pub fn new(indicator: Indicator) -> Self {
        let now = Instant::now();
        let duration = match indicator {
            Indicator::Media(_) => MEDIA_DISPLAY_DURATION,
            Indicator::Volume(_) | Indicator::Brightness(_) => DISPLAY_DURATION,
        };
        Self {
            indicator,
            until: now + duration,
            sampled_at: now,
        }
    }

    /// Whether the popup draws the wider now-playing card.
    fn is_media(&self) -> bool {
        matches!(self.indicator, Indicator::Media(_))
    }
}

/// Values drawn by the indicator for one level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IndicatorDisplay {
    icon: SetIcon,
    shown: u8,
    muted: bool,
}

fn indicator_display(indicator: Option<&Indicator>) -> IndicatorDisplay {
    match indicator {
        Some(Indicator::Volume(level)) => IndicatorDisplay {
            icon: SetIcon::Volume,
            shown: level.percent,
            muted: level.muted,
        },
        Some(Indicator::Brightness(percent)) => IndicatorDisplay {
            icon: SetIcon::Brightness,
            shown: *percent,
            muted: false,
        },
        Some(Indicator::Media(_)) | None => IndicatorDisplay {
            icon: SetIcon::Volume,
            shown: 0,
            muted: false,
        },
    }
}

/// Bounds of the indicator: horizontally centered, near the bottom edge of
/// the primary display. Falls back to a centered window without a display.
fn bounds(media: bool, cx: &App) -> Bounds<gpui::Pixels> {
    let (width, height) = if media {
        (MEDIA_WIDTH, MEDIA_HEIGHT)
    } else {
        (WIDTH, HEIGHT)
    };
    let window_size = size(px(width), px(height));
    match cx.primary_display().map(|display| display.bounds()) {
        Some(screen) => Bounds {
            origin: point(
                screen.center().x - px(width / 2.0),
                screen.bottom() - px(height + BOTTOM_MARGIN),
            ),
            size: window_size,
        },
        None => Bounds::centered(None, window_size, cx),
    }
}

/// Opens the indicator window sized for `media`, unless it is already open.
fn open(shared: &Shared, media: bool, cx: &mut App) {
    let state = shared.borrow();
    if state.osd_window.is_some()
        || state
            .osd_open_retry_at
            .is_some_and(|at| at.elapsed() < WINDOW_OPEN_RETRY)
    {
        return;
    }
    drop(state);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds(media, cx))),
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
    match cx.open_window(options, move |window, cx| {
        if let Err(error) = window_frame::strip_frame(&*window) {
            tracing::warn!(%error, "could not remove the indicator frame");
        }
        cx.new(move |_: &mut Context<OsdView>| OsdView {
            shared: view_shared.clone(),
            cover: None,
        })
    }) {
        Ok(handle) => {
            let mut state = shared.borrow_mut();
            state.osd_window = Some(handle);
            state.osd_window_media = media;
            state.osd_open_retry_at = None;
        }
        Err(error) => {
            tracing::warn!(%error, "indicator window unavailable; retrying later");
            shared.borrow_mut().osd_open_retry_at = Some(Instant::now());
        }
    }
}

/// Closes the indicator window if open.
fn close(shared: &Shared, cx: &mut App) {
    let handle = shared.borrow_mut().osd_window.take();
    shared.borrow_mut().osd_open_retry_at = None;
    shared.borrow_mut().osd_window_media = false;
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Expires the indicator, then keeps its window in sync with the shell state.
/// Called once per tick from the shell.
pub(crate) fn sync(cx: &mut App, shared: &Shared) {
    let (wanted_media, opened_media) = {
        let mut state = shared.borrow_mut();
        if state
            .osd
            .as_ref()
            .is_some_and(|osd| Instant::now() >= osd.until)
        {
            state.osd = None;
        }
        let wanted = state.osd.as_ref().map(Osd::is_media);
        let opened = state.osd_window.as_ref().map(|_| state.osd_window_media);
        (wanted, opened)
    };

    match (wanted_media, opened_media) {
        (Some(wanted), Some(opened)) if wanted == opened => {
            let handle = shared.borrow().osd_window;
            if let Some(handle) = handle {
                let _ = handle.update(cx, |_, _, cx| cx.notify());
            }
        }
        (Some(_), Some(_)) => {
            // The card changes shape (level bar versus track card): the
            // window was opened with fixed bounds, so it must be rebuilt.
            // Close only here: destroying and creating a window in the same
            // tick would make Win32 dispatch their messages inside the App
            // borrow held by the tick, which gpui reports as a re-entrant
            // mutable borrow. The next tick sees the missing window and
            // opens the right size, at most one frame later.
            close(shared, cx);
        }
        (Some(wanted), None) => open(shared, wanted, cx),
        (None, Some(_)) => close(shared, cx),
        (None, None) => {}
    }
}

/// Root view of the indicator window.
pub struct OsdView {
    /// Shared shell state (current indicator and theme).
    pub shared: Shared,
    /// Cover texture of the last shown track, reused across frames so the
    /// bytes are decoded once per track. Keyed by the full track view (title,
    /// artist, timing and cover bytes) so a different session with the same
    /// title cannot borrow the previous session's artwork.
    cover: Option<(PlayingView, Option<Arc<Image>>)>,
}

/// Texture for the cover of `track`, rebuilt into `cache` when the track
/// changes.
fn cover_texture(
    cache: &mut Option<(PlayingView, Option<Arc<Image>>)>,
    track: &PlayingView,
) -> Option<Arc<Image>> {
    let fresh = cache.as_ref().is_none_or(|(cached, _)| cached != track);
    if fresh {
        let image = track.cover.as_ref().map(|cover: &CoverView| {
            let format = match cover.format {
                CoverImageFormat::Png => ImageFormat::Png,
                CoverImageFormat::Jpeg => ImageFormat::Jpeg,
            };
            Arc::new(Image::from_bytes(format, cover.bytes.clone()))
        });
        *cache = Some((track.clone(), image));
    }
    cache
        .as_ref()
        .and_then(|(_, image)| image.as_ref().cloned())
}

impl Render for OsdView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.shared.borrow();
        let t = style::tokens(state.preferences.theme_mode);
        let card = match state.osd.as_ref() {
            Some(osd) => match &osd.indicator {
                Indicator::Media(track) => {
                    media_card(&mut self.cover, track, osd.sampled_at.elapsed(), &t)
                }
                indicator => level_card(Some(indicator), &t),
            },
            None => level_card(None, &t),
        };

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .font_family("Rubik")
            .text_size(px(14.0))
            .child(card)
    }
}

/// Volume or brightness bar card.
fn level_card(indicator: Option<&Indicator>, t: &style::Tokens) -> AnyElement {
    let display = indicator_display(indicator);
    let icon = if display.muted {
        iconset::lucide_element(LucideIcon::VolumeX, ICON_SIZE, t.text).into_any_element()
    } else {
        iconset::element(display.icon, ICON_SIZE, t.text).into_any_element()
    };
    let fraction = f32::from(display.shown) / 100.0;

    div()
        .size_full()
        .px(px(16.0))
        .rounded(px(style::RADIUS_CARD))
        .bg(t.card)
        .flex()
        .items_center()
        .gap_3()
        .child(icon)
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
                .child(display.shown.to_string()),
        )
        .into_any_element()
}

/// Now-playing card: cover, title, artist and playback state.
///
/// `since_sample` is the time elapsed since the session was read: while the
/// track plays, the shown position advances with the local clock, so the
/// bar and the elapsed time move while the card is on screen without
/// re-querying the player from the UI thread.
fn media_card(
    cache: &mut Option<(PlayingView, Option<Arc<Image>>)>,
    track: &PlayingView,
    since_sample: Duration,
    t: &style::Tokens,
) -> AnyElement {
    let cover = cover_texture(cache, track);
    let left = match cover {
        Some(image) => img(ImageSource::Image(image))
            .size(px(COVER_SIZE))
            .flex_shrink_0()
            .rounded(px(6.0))
            .into_any_element(),
        None => div()
            .size(px(COVER_SIZE))
            .flex_shrink_0()
            .rounded(px(6.0))
            .bg(t.border)
            .flex()
            .items_center()
            .justify_center()
            .child(iconset::element(SetIcon::Media, 22.0, t.muted))
            .into_any_element(),
    };
    // The glyph names the control the user would press next, like transport
    // buttons do: a session that is playing shows pause.
    let status = if track.playing {
        LucideIcon::Pause
    } else {
        LucideIcon::Play
    };

    div()
        .size_full()
        .px(px(6.0))
        .py(px(PADDING_VERTICAL))
        .rounded(px(style::RADIUS_CARD))
        .bg(t.card)
        .flex()
        .items_center()
        .gap_3()
        .child(left)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(t.text)
                        .text_ellipsis()
                        .child(track.title.clone()),
                )
                .when(!track.artist.is_empty(), |column| {
                    column.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(t.muted)
                            .text_ellipsis()
                            .child(track.artist.clone()),
                    )
                })
                .when(track.duration_ms > 0, |column| {
                    column.child(progress_row(track, shown_position(track, since_sample), t))
                }),
        )
        .child(
            div()
                .size(px(COVER_SIZE))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(iconset::lucide_element(status, 14.0, t.muted)),
        )
        .into_any_element()
}

/// Position to draw for `track` when `since_sample` has elapsed since the
/// session read: playing advances on the local clock, paused holds, and the
/// result never passes the end of the track.
fn shown_position(track: &PlayingView, since_sample: Duration) -> u64 {
    if !track.playing {
        return track.position_ms;
    }
    let advanced = track.position_ms + since_sample.as_millis() as u64;
    if track.duration_ms > 0 {
        advanced.min(track.duration_ms)
    } else {
        advanced
    }
}

/// Elapsed, bar and total length of the track, as one compact row.
fn progress_row(track: &PlayingView, shown_ms: u64, t: &style::Tokens) -> AnyElement {
    let fraction = if track.duration_ms == 0 {
        0.0
    } else {
        (shown_ms as f32 / track.duration_ms as f32).clamp(0.0, 1.0)
    };

    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(10.0))
        .text_color(t.muted)
        .child(clock(shown_ms))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .h(px(4.0))
                .rounded_full()
                .bg(t.border)
                .child(
                    div()
                        .h_full()
                        .w(relative(fraction))
                        .rounded_full()
                        .bg(style::ACCENT),
                ),
        )
        .child(clock(track.duration_ms))
        .into_any_element()
}

/// Milliseconds as `m:ss`, or `h:mm:ss` past one hour.
fn clock(milliseconds: u64) -> String {
    let total = milliseconds / 1000;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::VolumeView;

    fn track() -> PlayingView {
        PlayingView {
            title: "Nightcall".to_owned(),
            artist: "Kavinsky".to_owned(),
            cover: None,
            playing: true,
            position_ms: 12_000,
            duration_ms: 240_000,
        }
    }

    #[test]
    fn muted_volume_keeps_the_percentage() {
        let display = indicator_display(Some(&Indicator::Volume(VolumeView {
            percent: 42,
            muted: true,
        })));

        assert_eq!(display.icon, SetIcon::Volume);
        assert_eq!(display.shown, 42);
        assert!(display.muted);
    }

    #[test]
    fn unmuted_volume_uses_the_normal_icon() {
        let display = indicator_display(Some(&Indicator::Volume(VolumeView {
            percent: 75,
            muted: false,
        })));

        assert_eq!(display.icon, SetIcon::Volume);
        assert_eq!(display.shown, 75);
        assert!(!display.muted);
    }

    #[test]
    fn brightness_has_no_muted_state() {
        let display = indicator_display(Some(&Indicator::Brightness(60)));

        assert_eq!(display.icon, SetIcon::Brightness);
        assert_eq!(display.shown, 60);
        assert!(!display.muted);
    }

    #[test]
    fn media_indicators_stay_longer_than_levels() {
        let track = Indicator::Media(track());
        let volume = Indicator::Volume(VolumeView {
            percent: 30,
            muted: false,
        });

        assert!(Osd::new(track).until > Osd::new(volume).until);
    }

    /// The cache key includes the whole track view, so a new session with the
    /// same title and artist cannot reuse another session's cover.
    #[test]
    fn cover_cache_key_includes_session_state() {
        let first = track();
        let mut second = track();
        second.position_ms += 1;
        assert_ne!(first, second);
    }

    #[test]
    fn clock_formats_positions() {
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(75_000), "1:15");
        assert_eq!(clock(3_723_000), "1:02:03");
    }

    #[test]
    fn a_playing_track_advances_with_the_local_clock() {
        let track = PlayingView {
            position_ms: 10_000,
            duration_ms: 60_000,
            playing: true,
            ..track()
        };

        assert_eq!(shown_position(&track, Duration::from_millis(5_000)), 15_000);
        assert_eq!(shown_position(&track, Duration::from_secs(300)), 60_000);
    }

    #[test]
    fn a_paused_track_holds_its_position() {
        let track = PlayingView {
            position_ms: 10_000,
            duration_ms: 60_000,
            playing: false,
            ..track()
        };

        assert_eq!(shown_position(&track, Duration::from_secs(1)), 10_000);
    }

    #[test]
    fn only_media_indicators_widen_the_window() {
        let volume = Osd::new(Indicator::Volume(VolumeView {
            percent: 30,
            muted: false,
        }));
        let track = Osd::new(Indicator::Media(track()));

        assert!(!volume.is_media());
        assert!(track.is_media());
    }
}
