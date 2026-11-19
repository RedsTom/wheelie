//! Settings page: appearance, language and information about Wheelie.

use super::{info_row, platform_notice_text, separator};
use crate::core::{APP_NAME, Language, TextKey, ThemeMode, VERSION, tr};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
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
    shared: &Shared,
    _window: &Window,
    _cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;

    div()
        .id("settings-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .overflow_scroll()
        .gap(px(8.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabSettings),
            tr(language, TextKey::SettingsSubtitle),
        ))
        .child(preferences_card(state, shared, t, language))
        .child(about_card(state, t, language))
        .child(platform_card(state, t, language))
        .child(open_source_card(t, language))
}

fn preferences_card(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
) -> impl IntoElement {
    let open = state.page.open_menu.as_deref();
    let theme = state.preferences.theme_mode;
    let native = state.preferences.language;

    let theme_options = ThemeMode::ALL
        .iter()
        .map(|mode| theme_label(language, *mode).to_owned())
        .collect::<Vec<_>>();
    let theme_selected = ThemeMode::ALL
        .iter()
        .position(|mode| *mode == theme)
        .unwrap_or(0);

    let language_options = Language::ALL
        .iter()
        .map(|lang| lang.native_name().to_owned())
        .collect::<Vec<_>>();
    let language_selected = Language::ALL
        .iter()
        .position(|lang| *lang == native)
        .unwrap_or(0);

    let start_at_login = state
        .runtime
        .start_at_login_enabled()
        .unwrap_or(state.runtime.config().general.start_at_login);

    let theme_shared = shared.clone();
    let theme_pick = shared.clone();
    let language_shared = shared.clone();
    let language_pick = shared.clone();
    let login_runtime = state.runtime.clone();

    controls::card(
        "settings-preferences",
        t,
        div()
            .flex()
            .flex_col()
            .child(controls::section_title(
                "settings-section-preferences",
                t,
                tr(language, TextKey::PreferencesTitle),
                Some(tr(language, TextKey::PreferencesBody)),
            ))
            .child(
                div().mt(px(8.0)).child(controls::setting_row(
                    "settings-row-theme",
                    t,
                    tr(language, TextKey::ThemeLabel),
                    tr(language, TextKey::ThemeDescription),
                    div()
                        .relative()
                        .child(controls::select_button(
                            "theme",
                            t,
                            theme_label(language, theme),
                            open == Some("theme"),
                            130.0,
                            move |_, _, _| {
                                let mut page = theme_shared.borrow_mut();
                                page.page.open_menu =
                                    if page.page.open_menu.as_deref() == Some("theme") {
                                        None
                                    } else {
                                        Some("theme".to_owned())
                                    };
                            },
                        ))
                        .when(open == Some("theme"), |div| {
                            div.child(controls::select_menu(
                                "theme",
                                t,
                                &theme_options,
                                theme_selected,
                                130.0,
                                move |index, _, _| {
                                    let mut state = theme_pick.borrow_mut();
                                    state.preferences.theme_mode = ThemeMode::ALL[index];
                                    state.page.open_menu = None;
                                },
                            ))
                        }),
                )),
            )
            .child(div().my(px(6.0)).child(separator(t)))
            .child(controls::setting_row(
                "settings-row-language",
                t,
                tr(language, TextKey::LanguageLabel),
                tr(language, TextKey::LanguageDescription),
                div()
                    .relative()
                    .child(controls::select_button(
                        "language",
                        t,
                        native.native_name(),
                        open == Some("language"),
                        130.0,
                        move |_, _, _| {
                            let mut page = language_shared.borrow_mut();
                            page.page.open_menu =
                                if page.page.open_menu.as_deref() == Some("language") {
                                    None
                                } else {
                                    Some("language".to_owned())
                                };
                        },
                    ))
                    .when(open == Some("language"), |div| {
                        div.child(controls::select_menu(
                            "language",
                            t,
                            &language_options,
                            language_selected,
                            130.0,
                            move |index, _, _| {
                                let mut state = language_pick.borrow_mut();
                                state.preferences.language = Language::ALL[index];
                                state.page.open_menu = None;
                            },
                        ))
                    }),
            ))
            .child(div().my(px(6.0)).child(separator(t)))
            .child(controls::setting_row(
                "settings-row-autostart",
                t,
                tr(language, TextKey::StartAtLoginLabel),
                tr(language, TextKey::StartAtLoginDescription),
                controls::toggle("autostart", start_at_login, move |_checked, _, _| {
                    let enabled = !start_at_login;
                    match login_runtime.set_start_at_login(enabled) {
                        Ok(()) => {
                            if let Err(error) = login_runtime.update_config(|config| {
                                config.general.start_at_login = enabled;
                            }) {
                                login_runtime.report_error(error.to_string());
                            }
                        }
                        Err(error) => login_runtime.report_error(error.to_string()),
                    }
                }),
            )),
    )
}

fn about_card(state: &ShellState, t: Tokens, language: Language) -> impl IntoElement {
    controls::card(
        "settings-about",
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
    )
}

fn platform_card(state: &ShellState, t: Tokens, language: Language) -> impl IntoElement {
    controls::card(
        "settings-platform",
        t,
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(controls::section_title(
                "settings-section-platform",
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
    )
}

fn open_source_card(t: Tokens, language: Language) -> impl IntoElement {
    controls::card(
        "settings-open-source",
        t,
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(controls::section_title(
                "settings-section-open-source",
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
    )
}

fn theme_label(language: Language, mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Dark => tr(language, TextKey::DarkTheme),
        ThemeMode::Light => tr(language, TextKey::LightTheme),
    }
}
