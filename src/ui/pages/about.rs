//! About page.

use super::{info_row, platform_notice_text, separator};
use crate::core::{APP_NAME, TextKey, VERSION, tr};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style;
use gpui::prelude::*;
use gpui::{Context, Window, div, img, px};

/// Libraries Wheelie is built on, with their source repositories.
const OPEN_SOURCE: &[(&str, &str)] = &[
    ("gpui-kit", "https://github.com/longbridge/gpui-kit"),
    ("GPUI", "https://github.com/zed-industries/zed"),
    ("gpui-icons", "https://github.com/karyalabs/gpui-icons"),
    ("enigo", "https://github.com/enigo-rs/enigo"),
    (
        "global-hotkey",
        "https://github.com/tauri-apps/global-hotkey",
    ),
    ("tray-icon", "https://github.com/tauri-apps/tray-icon"),
    ("serde", "https://github.com/serde-rs/serde"),
    ("toml", "https://github.com/toml-rs/toml"),
    ("tracing", "https://github.com/tokio-rs/tracing"),
];

pub(crate) fn render(
    state: &ShellState,
    _shared: &Shared,
    _window: &Window,
    _cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;

    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::AboutTitle),
            tr(language, TextKey::AboutSubtitle),
        ))
        .child(controls::card(
            t,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(img("logo/logo-2x.png").size(px(56.0)).rounded(px(12.0)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_size(px(20.0))
                                        .text_color(t.text)
                                        .child(APP_NAME),
                                )
                                .child(
                                    div()
                                        .mt(px(2.0))
                                        .text_size(px(12.5))
                                        .text_color(t.muted)
                                        .child(tr(language, TextKey::AppTagline)),
                                )
                                .child(div().mt(px(6.0)).child(controls::status_pill(
                                    tr(language, TextKey::FreeSoftware),
                                    style::ACCENT,
                                ))),
                        ),
                )
                .child(
                    div()
                        .mt(px(6.0))
                        .text_size(px(13.0))
                        .text_color(t.muted)
                        .child(tr(language, TextKey::AboutBody)),
                )
                .child(div().mt(px(8.0)).child(separator(t)))
                .child(info_row(t, tr(language, TextKey::VersionLabel), VERSION))
                .child(info_row(
                    t,
                    tr(language, TextKey::LicenseLabel),
                    "GPL-3.0-only",
                ))
                .child(info_row(
                    t,
                    tr(language, TextKey::SettingsFileLabel),
                    &state.runtime.config_path().display().to_string(),
                )),
        ))
        .child(controls::card(
            t,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(controls::section_title(
                    t,
                    tr(language, TextKey::PlatformCardTitle),
                    None,
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .child(controls::status_pill(state.platform.os, style::ACCENT)),
                )
                .child(div().when_some(
                    platform_notice_text(language, state.platform),
                    |div, notice| div.child(super::warning(notice)),
                )),
        ))
        .child(controls::card(
            t,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(controls::section_title(
                    t,
                    tr(language, TextKey::OpenSourceTitle),
                    Some(tr(language, TextKey::OpenSourceBody)),
                ))
                .child(div().mt(px(4.0)).flex().flex_col().children(
                    OPEN_SOURCE.iter().enumerate().map(|(index, (name, url))| {
                        let url = *url;
                        div()
                            .id(format!("open-source-{index}"))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_2()
                            .py(px(6.0))
                            .rounded(px(style::RADIUS_CONTROL))
                            .cursor_pointer()
                            .hover(|node| node.bg(t.card_hover))
                            .on_click(move |_, _, cx| cx.open_url(url))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(style::ACCENT)
                                    .child(*name),
                            )
                            .child(
                                gpui_icons::lucide(gpui_icons::LucideIcon::ExternalLink)
                                    .size(px(14.0))
                                    .text_color(t.muted),
                            )
                    }),
                )),
        ))
}
