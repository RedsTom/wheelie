//! General settings page.

use super::state_label;
use crate::app::{AppCommand, RuntimeState};
use crate::core::{Language, TextKey, ThemeMode, tr};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{Context, Window, div, img, px};

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    _cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;
    let wide = window.viewport_size().width >= px(840.0);

    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabGeneral),
            tr(language, TextKey::GeneralSubtitle),
        ))
        .child(runtime_card(state, t, language))
        .child(
            div()
                .flex()
                .when(wide, |node| node.flex_row().items_start().gap(px(14.0)))
                .when(!wide, |node| node.flex_col().gap(px(14.0)))
                .child(
                    div()
                        .w_full()
                        .child(preferences_card(state, shared, t, language)),
                )
                .child(div().w_full().child(getting_started_card(t, language))),
        )
}

fn runtime_card(state: &ShellState, t: Tokens, language: Language) -> impl IntoElement {
    let snapshot = &state.snapshot;
    let title = match snapshot.state {
        RuntimeState::Running | RuntimeState::Starting => TextKey::RuntimeReadyTitle,
        RuntimeState::Paused => TextKey::RuntimePausedTitle,
        RuntimeState::Stopped => TextKey::RuntimeStoppedTitle,
    };
    let body = match snapshot.state {
        RuntimeState::Running | RuntimeState::Starting => TextKey::RuntimeReadyBody,
        RuntimeState::Paused => TextKey::RuntimePausedBody,
        RuntimeState::Stopped => TextKey::RuntimeStoppedBody,
    };
    let (state_text, state_color) = state_label(language, snapshot.state);
    let stopped = matches!(snapshot.state, RuntimeState::Stopped);
    let resume = matches!(snapshot.state, RuntimeState::Paused);
    let error = snapshot.last_error.clone();

    let runtime = state.runtime.clone();

    controls::card(
        t,
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(img("logo/logo-2x.png").size(px(44.0)).rounded(px(10.0)))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_size(px(18.0))
                                            .text_color(t.text)
                                            .child(tr(language, title)),
                                    )
                                    .child(
                                        div()
                                            .mt(px(2.0))
                                            .text_size(px(13.0))
                                            .text_color(t.muted)
                                            .child(tr(language, body)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(if stopped {
                                controls::disabled_button(t, pause_label(language, resume))
                                    .into_any_element()
                            } else {
                                let command = if resume {
                                    AppCommand::Resume
                                } else {
                                    AppCommand::Pause
                                };
                                controls::primary_button(
                                    pause_label(language, resume),
                                    move |_, _, _| {
                                        let _ = runtime.send(command);
                                    },
                                )
                                .into_any_element()
                            })
                            .child(controls::status_pill(state_text, state_color)),
                    ),
            )
            .when_some(error, |section, error| {
                section.child(
                    div()
                        .text_size(px(12.5))
                        .text_color(style::DANGER)
                        .child(format!(
                            "{}: {error}",
                            tr(language, TextKey::LastErrorLabel)
                        )),
                )
            }),
    )
}

fn pause_label(language: Language, resume: bool) -> &'static str {
    tr(
        language,
        if resume {
            TextKey::ResumeAction
        } else {
            TextKey::PauseAction
        },
    )
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
        t,
        div()
            .flex()
            .flex_col()
            .child(controls::section_title(
                t,
                tr(language, TextKey::PreferencesTitle),
                Some(tr(language, TextKey::PreferencesBody)),
            ))
            .child(
                div().mt(px(10.0)).child(controls::setting_row(
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
                                move |index, _, _| {
                                    let mut state = theme_pick.borrow_mut();
                                    state.preferences.theme_mode = ThemeMode::ALL[index];
                                    state.page.open_menu = None;
                                },
                            ))
                        }),
                )),
            )
            .child(div().my(px(8.0)).child(super::separator(t)))
            .child(controls::setting_row(
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
                            move |index, _, _| {
                                let mut state = language_pick.borrow_mut();
                                state.preferences.language = Language::ALL[index];
                                state.page.open_menu = None;
                            },
                        ))
                    }),
            ))
            .child(div().my(px(8.0)).child(super::separator(t)))
            .child(controls::setting_row(
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

fn getting_started_card(t: Tokens, language: Language) -> impl IntoElement {
    controls::card(
        t,
        div()
            .flex()
            .flex_col()
            .child(controls::section_title(
                t,
                tr(language, TextKey::GettingStartedTitle),
                Some(tr(language, TextKey::GettingStartedSubtitle)),
            ))
            .child(div().mt(px(10.0)).flex().flex_col().gap(px(8.0)).children([
                controls::timeline_item(
                    t,
                    1,
                    tr(language, TextKey::StepConfigureTitle),
                    tr(language, TextKey::StepConfigureBody),
                    true,
                ),
                controls::timeline_item(
                    t,
                    2,
                    tr(language, TextKey::StepAssignTitle),
                    tr(language, TextKey::StepAssignBody),
                    false,
                ),
                controls::timeline_item(
                    t,
                    3,
                    tr(language, TextKey::StepOpenTitle),
                    tr(language, TextKey::StepOpenBody),
                    false,
                ),
            ])),
    )
}

fn theme_label(language: Language, mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Dark => tr(language, TextKey::DarkTheme),
        ThemeMode::Light => tr(language, TextKey::LightTheme),
    }
}
