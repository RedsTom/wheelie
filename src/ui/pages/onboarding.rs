//! First-launch setup shown instead of the settings pages.
//!
//! Three short pages pick the interface language and start-at-login, assign the
//! three knob keys and explain how the wheel is used. The pages reuse the
//! settings widgets and the shared key-capture listener, so no second window is
//! needed.

use super::{devices, separator};
use crate::core::{Language, TextKey, tr};
use crate::ui::controls;
use crate::ui::settings::active_profile_index;
use crate::ui::shell::{OnboardingStep, SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, Context, Rgba, Window, div, img, px};

const LANGUAGE_MENU: &str = "onboarding-language";

/// Renders the current setup page.
pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    _window: &mut Window,
    _cx: &mut Context<SettingsView>,
) -> AnyElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;
    let step = state.onboarding.step;

    let content = match step {
        OnboardingStep::Language => language_page(state, t, language, shared),
        OnboardingStep::Keys => keys_page(state, shared, t, language),
        OnboardingStep::Usage => usage_page(t, language),
    };

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(header(t, language, step))
        .child(content)
        .child(footer(t, language, shared))
        .into_any_element()
}

fn header(t: Tokens, language: Language, step: OnboardingStep) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
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
                                .text_size(px(19.0))
                                .text_color(t.text)
                                .child(tr(language, TextKey::OnboardingTitle)),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(12.5))
                                .text_color(t.muted)
                                .child(tr(language, TextKey::OnboardingSubtitle)),
                        ),
                ),
        )
        .child(stepper(t, language, step))
        .into_any_element()
}

fn stepper(t: Tokens, language: Language, current: OnboardingStep) -> AnyElement {
    let steps = [
        (
            OnboardingStep::Language,
            tr(language, TextKey::OnboardingLanguageTitle),
        ),
        (
            OnboardingStep::Keys,
            tr(language, TextKey::OnboardingKeysTitle),
        ),
        (
            OnboardingStep::Usage,
            tr(language, TextKey::OnboardingUsageTitle),
        ),
    ];
    let current_index = OnboardingStep::ALL
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);

    let mut row = div().flex().items_center().gap_2();
    for (index, (step, label)) in steps.iter().enumerate() {
        let reached = *step == current || index < current_index;
        let accent = if reached { style::ACCENT } else { t.border };
        let text = if reached { t.text } else { t.muted };

        row = row.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .rounded_full()
                        .bg(Rgba { a: 0.16, ..accent })
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(accent)
                        .child((index + 1).to_string()),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(text)
                        .child(label.to_string()),
                ),
        );

        if index + 1 < steps.len() {
            row = row.child(div().h(px(1.0)).w(px(20.0)).bg(t.border));
        }
    }
    row.into_any_element()
}

fn language_page(state: &ShellState, t: Tokens, language: Language, shared: &Shared) -> AnyElement {
    let open = shared.borrow().page.open_menu.as_deref() == Some(LANGUAGE_MENU);
    let options = Language::ALL
        .iter()
        .map(|candidate| candidate.native_name().to_owned())
        .collect::<Vec<_>>();
    let selected = Language::ALL
        .iter()
        .position(|candidate| *candidate == language)
        .unwrap_or(0);
    let start_at_login = state
        .runtime
        .start_at_login_enabled()
        .unwrap_or(state.runtime.config().general.start_at_login);

    let toggle_shared = shared.clone();
    let pick_shared = shared.clone();
    let login_runtime = state.runtime.clone();

    controls::card(
        "onboarding-language",
        t,
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(controls::section_title(
                "onboarding-language-title",
                t,
                tr(language, TextKey::OnboardingLanguageTitle),
                Some(tr(language, TextKey::OnboardingLanguageBody)),
            ))
            .child(controls::setting_row(
                "onboarding-language-row",
                t,
                tr(language, TextKey::LanguageLabel),
                tr(language, TextKey::LanguageDescription),
                div()
                    .relative()
                    .child(controls::select_button(
                        LANGUAGE_MENU,
                        t,
                        language.native_name(),
                        open,
                        140.0,
                        move |_, _, _| {
                            let mut state = toggle_shared.borrow_mut();
                            state.page.open_menu =
                                if state.page.open_menu.as_deref() == Some(LANGUAGE_MENU) {
                                    None
                                } else {
                                    Some(LANGUAGE_MENU.to_owned())
                                };
                        },
                    ))
                    .when(open, |node| {
                        node.child(controls::select_menu(
                            LANGUAGE_MENU,
                            t,
                            &options,
                            selected,
                            140.0,
                            move |index, _, _| {
                                let mut state = pick_shared.borrow_mut();
                                state.preferences.language = Language::ALL[index];
                                state.page.open_menu = None;
                            },
                        ))
                    }),
            ))
            .child(separator(t))
            .child(controls::setting_row(
                "onboarding-autostart-row",
                t,
                tr(language, TextKey::StartAtLoginLabel),
                tr(language, TextKey::StartAtLoginDescription),
                controls::toggle(
                    "onboarding-autostart",
                    start_at_login,
                    move |_checked, _, _| {
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
                    },
                ),
            )),
    )
    .into_any_element()
}

fn keys_page(state: &ShellState, shared: &Shared, t: Tokens, language: Language) -> AnyElement {
    let config = state.runtime.config();
    let mut body = div().flex().flex_col().gap(px(10.0));

    if let Some(notice) = super::platform_notice_text(language, state.platform) {
        body = body.child(super::warning(notice));
    }
    if let Some(index) = active_profile_index(&config) {
        body = body.child(devices::bindings_card(
            state,
            shared,
            t,
            language,
            config.profiles[index].bindings,
        ));
    }

    body.child(
        div()
            .text_size(px(12.5))
            .text_color(t.muted)
            .child(tr(language, TextKey::OnboardingKeysHint)),
    )
    .into_any_element()
}

fn usage_page(t: Tokens, language: Language) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(controls::section_title(
            "onboarding-usage-title",
            t,
            tr(language, TextKey::OnboardingUsageTitle),
            None,
        ))
        .child(controls::timeline_item(
            t,
            1,
            tr(language, TextKey::OnboardingUsageDoubleTitle),
            tr(language, TextKey::OnboardingUsageDoubleBody),
            false,
        ))
        .child(controls::timeline_item(
            t,
            2,
            tr(language, TextKey::OnboardingUsageChooseTitle),
            tr(language, TextKey::OnboardingUsageChooseBody),
            false,
        ))
        .child(controls::timeline_item(
            t,
            3,
            tr(language, TextKey::OnboardingUsageRunTitle),
            tr(language, TextKey::OnboardingUsageRunBody),
            false,
        ))
        .child(separator(t))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(t.muted)
                .child(tr(language, TextKey::OnboardingUsageInvite)),
        );

    controls::card("onboarding-usage", t, body).into_any_element()
}

fn footer(t: Tokens, language: Language, shared: &Shared) -> AnyElement {
    let step = shared.borrow().onboarding.step;
    let back_shared = shared.clone();
    let next_shared = shared.clone();
    let finish_shared = shared.clone();

    let back = if step == OnboardingStep::Language {
        div().into_any_element()
    } else {
        controls::secondary_button(t, tr(language, TextKey::OnboardingBack), move |_, _, _| {
            retreat(&back_shared)
        })
        .into_any_element()
    };

    let forward: AnyElement = if step == OnboardingStep::Usage {
        controls::primary_button(tr(language, TextKey::OnboardingFinish), move |_, _, _| {
            finish(&finish_shared)
        })
        .into_any_element()
    } else {
        controls::primary_button(tr(language, TextKey::OnboardingNext), move |_, _, _| {
            advance(&next_shared)
        })
        .into_any_element()
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .pt(px(4.0))
        .child(back)
        .child(forward)
        .into_any_element()
}

fn advance(shared: &Shared) {
    let mut state = shared.borrow_mut();
    clear_capture(&mut state);
    state.onboarding.step = next_step(state.onboarding.step);
}

fn retreat(shared: &Shared) {
    let mut state = shared.borrow_mut();
    clear_capture(&mut state);
    state.onboarding.step = previous_step(state.onboarding.step);
}

/// Persists completion and returns the user to the settings window.
fn finish(shared: &Shared) {
    let runtime = shared.borrow().runtime.clone();
    if let Err(error) = runtime.complete_onboarding() {
        runtime.report_error(error.to_string());
        return;
    }

    let mut state = shared.borrow_mut();
    clear_capture(&mut state);
    state.onboarding_active = false;
    state.page.open_menu = None;
}

fn clear_capture(state: &mut ShellState) {
    state.capture = None;
    state.capture_pending = None;
}

fn next_step(step: OnboardingStep) -> OnboardingStep {
    let index = OnboardingStep::ALL
        .iter()
        .position(|candidate| *candidate == step)
        .unwrap_or(0);
    OnboardingStep::ALL[(index + 1).min(OnboardingStep::ALL.len() - 1)]
}

fn previous_step(step: OnboardingStep) -> OnboardingStep {
    let index = OnboardingStep::ALL
        .iter()
        .position(|candidate| *candidate == step)
        .unwrap_or(0);
    OnboardingStep::ALL[index.saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_page_picks_the_language() {
        assert_eq!(OnboardingStep::default(), OnboardingStep::Language);
        assert_eq!(OnboardingStep::ALL.len(), 3);
    }

    #[test]
    fn autostart_labels_are_available_in_both_languages() {
        let fr_label = tr(Language::Fr, TextKey::StartAtLoginLabel);
        let en_label = tr(Language::En, TextKey::StartAtLoginLabel);
        assert!(!fr_label.is_empty());
        assert!(!en_label.is_empty());
        assert_ne!(fr_label, en_label);
    }

    #[test]
    fn steps_advance_and_stop_at_the_end() {
        assert_eq!(next_step(OnboardingStep::Language), OnboardingStep::Keys);
        assert_eq!(next_step(OnboardingStep::Keys), OnboardingStep::Usage);
        assert_eq!(next_step(OnboardingStep::Usage), OnboardingStep::Usage);
    }

    #[test]
    fn steps_go_back_and_stop_at_the_start() {
        assert_eq!(previous_step(OnboardingStep::Usage), OnboardingStep::Keys);
        assert_eq!(
            previous_step(OnboardingStep::Keys),
            OnboardingStep::Language
        );
        assert_eq!(
            previous_step(OnboardingStep::Language),
            OnboardingStep::Language
        );
    }

    #[test]
    fn step_numbers_are_one_based() {
        assert_eq!(OnboardingStep::Language.number(), 1);
        assert_eq!(OnboardingStep::Usage.number(), OnboardingStep::ALL.len());
    }
}
