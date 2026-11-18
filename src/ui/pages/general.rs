//! Overview page: status at a glance and a read-only wheel preview.

use super::state_label;
use crate::app::{AppCommand, RuntimeState};
use crate::core::{Language, TextKey, WheelGeometry, tr};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use crate::ui::wheel_paint;
use gpui::prelude::*;
use gpui::{Context, Window, canvas, div, img, point, px};

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
        .gap(px(8.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabGeneral),
            tr(language, TextKey::GeneralSubtitle),
        ))
        .child(runtime_card(state, t, language))
        .child(wheel_card(state, t, language))
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
        "overview-status",
        t,
        div()
            .flex()
            .flex_col()
            .gap_2()
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

/// Read-only wheel showing the active action set. Selecting a set is done on
/// the Actions page; this card only reflects the current choice.
fn wheel_card(state: &ShellState, t: Tokens, language: Language) -> impl IntoElement {
    const BOX: f32 = 280.0;

    let config = state.runtime.config();
    let sets = config.action_sets.clone();
    let selected = config
        .active_set
        .as_deref()
        .and_then(|id| sets.iter().position(|set| set.id == id))
        .or(if sets.is_empty() { None } else { Some(0) })
        .unwrap_or(0);

    let geometry = WheelGeometry::from_size(BOX);
    let center = point(px(BOX / 2.0), px(BOX / 2.0));
    let origin = wheel_paint::origin(center, &geometry);
    let paint_sets = sets.clone();
    let title_y = BOX - 8.0;

    controls::card(
        "overview-wheel",
        t,
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(controls::section_title(
                t,
                tr(language, TextKey::WheelPreviewTitle),
                Some(tr(language, TextKey::WheelPreviewBody)),
            ))
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .relative()
                        .w(px(BOX))
                        .h(px(BOX))
                        .child(
                            canvas(
                                |_, _, _| (),
                                move |bounds, _, window, _| {
                                    let paint_center = point(
                                        bounds.origin.x + px(BOX / 2.0),
                                        bounds.origin.y + px(BOX / 2.0),
                                    );
                                    wheel_paint::paint(
                                        window,
                                        &geometry,
                                        paint_center,
                                        &paint_sets,
                                        selected,
                                        None,
                                    );
                                },
                            )
                            .absolute()
                            .inset_0(),
                        )
                        .children(wheel_paint::children(
                            &geometry, origin, title_y, BOX, &sets, selected, true,
                        )),
                ),
            ),
    )
}
