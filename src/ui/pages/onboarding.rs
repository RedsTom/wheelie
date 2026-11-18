//! First-launch introduction shown instead of the settings pages.
//!
//! Four short pages explain the knob workflow, choose the interface language,
//! assign the three keys and confirm the result. The pages reuse the settings
//! widgets and the shared key-capture listener, so no second window is needed.

use super::{devices, info_row, separator};
use crate::core::{Language, TextKey, tr};
use crate::ui::controls;
use crate::ui::settings::active_profile_index;
use crate::ui::shell::{OnboardingStep, SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, Context, Window, div, img, px};

const LANGUAGE_MENU: &str = "onboarding-language";

/// Renders the current introduction page.
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
        OnboardingStep::Welcome => welcome(t, language),
        OnboardingStep::Language => language_page(t, language, shared),
        OnboardingStep::Keys => keys_page(state, shared, t, language),
        OnboardingStep::Done => done_page(state, t, language),
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
    let progress = format!("{} / {}", step.number(), OnboardingStep::ALL.len());
    let title = match step {
        OnboardingStep::Welcome => tr(language, TextKey::GettingStartedTitle),
        OnboardingStep::Language => tr(language, TextKey::OnboardingLanguageTitle),
        OnboardingStep::Keys => tr(language, TextKey::OnboardingKeysTitle),
        OnboardingStep::Done => tr(language, TextKey::OnboardingDoneTitle),
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
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
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(controls::inline_pill(&progress, style::ACCENT))
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(t.muted)
                        .child(title.to_owned()),
                ),
        )
        .into_any_element()
}

fn welcome(t: Tokens, language: Language) -> AnyElement {
    controls::card(
        "onboarding-welcome",
        t,
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(controls::section_title(
                "onboarding-welcome-title",
                t,
                tr(language, TextKey::GettingStartedTitle),
                Some(tr(language, TextKey::GettingStartedSubtitle)),
            ))
            .child(separator(t))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(controls::timeline_item(
                        t,
                        1,
                        tr(language, TextKey::StepConfigureTitle),
                        tr(language, TextKey::StepConfigureBody),
                        true,
                    ))
                    .child(controls::timeline_item(
                        t,
                        2,
                        tr(language, TextKey::StepAssignTitle),
                        tr(language, TextKey::StepAssignBody),
                        false,
                    ))
                    .child(controls::timeline_item(
                        t,
                        3,
                        tr(language, TextKey::StepOpenTitle),
                        tr(language, TextKey::StepOpenBody),
                        false,
                    )),
            ),
    )
    .into_any_element()
}

fn language_page(t: Tokens, language: Language, shared: &Shared) -> AnyElement {
    let open = shared.borrow().page.open_menu.as_deref() == Some(LANGUAGE_MENU);
    let options = Language::ALL
        .iter()
        .map(|candidate| candidate.native_name().to_owned())
        .collect::<Vec<_>>();
    let selected = Language::ALL
        .iter()
        .position(|candidate| *candidate == language)
        .unwrap_or(0);

    let toggle_shared = shared.clone();
    let pick_shared = shared.clone();

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
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(t.muted)
                    .child(tr(language, TextKey::GettingStartedSubtitle).to_owned()),
            ),
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

fn done_page(state: &ShellState, t: Tokens, language: Language) -> AnyElement {
    let config = state.runtime.config();
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(controls::section_title(
            "onboarding-done-title",
            t,
            tr(language, TextKey::OnboardingDoneTitle),
            None,
        ))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(t.muted)
                .child(tr(language, TextKey::OnboardingDoneBody)),
        )
        .child(separator(t))
        .child(info_row(
            t,
            tr(language, TextKey::LanguageLabel),
            language.native_name(),
        ));

    if let Some(index) = active_profile_index(&config) {
        let bindings = config.profiles[index].bindings;
        let left = bindings.left.to_string();
        let right = bindings.right.to_string();
        let press = bindings.press.to_string();
        body = body
            .child(info_row(t, tr(language, TextKey::ControlLeft), &left))
            .child(info_row(t, tr(language, TextKey::ControlRight), &right))
            .child(info_row(t, tr(language, TextKey::ControlPress), &press));
    }

    controls::card("onboarding-done", t, body).into_any_element()
}

fn footer(t: Tokens, language: Language, shared: &Shared) -> AnyElement {
    let step = shared.borrow().onboarding.step;
    let back_shared = shared.clone();
    let next_shared = shared.clone();
    let finish_shared = shared.clone();

    let back = if step == OnboardingStep::Welcome {
        div().into_any_element()
    } else {
        controls::secondary_button(t, tr(language, TextKey::OnboardingBack), move |_, _, _| {
            retreat(&back_shared)
        })
        .into_any_element()
    };

    let forward: AnyElement = if step == OnboardingStep::Done {
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
    fn steps_advance_and_stop_at_the_end() {
        assert_eq!(next_step(OnboardingStep::Welcome), OnboardingStep::Language);
        assert_eq!(next_step(OnboardingStep::Language), OnboardingStep::Keys);
        assert_eq!(next_step(OnboardingStep::Keys), OnboardingStep::Done);
        assert_eq!(next_step(OnboardingStep::Done), OnboardingStep::Done);
    }

    #[test]
    fn steps_go_back_and_stop_at_the_start() {
        assert_eq!(previous_step(OnboardingStep::Done), OnboardingStep::Keys);
        assert_eq!(
            previous_step(OnboardingStep::Keys),
            OnboardingStep::Language
        );
        assert_eq!(
            previous_step(OnboardingStep::Language),
            OnboardingStep::Welcome
        );
        assert_eq!(
            previous_step(OnboardingStep::Welcome),
            OnboardingStep::Welcome
        );
    }

    #[test]
    fn step_numbers_are_one_based() {
        assert_eq!(OnboardingStep::Welcome.number(), 1);
        assert_eq!(OnboardingStep::Done.number(), OnboardingStep::ALL.len());
    }
}
