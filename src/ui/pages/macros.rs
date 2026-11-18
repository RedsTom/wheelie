//! Macros page: list and edit reusable input sequences (gpui port).

use crate::app::RuntimeHandle;
use crate::core::action::{MouseButton, ScrollAxis};
use crate::core::{Config, KeyChord, KeyCode, Language, MacroDefinition, MacroStep, TextKey, tr};
use crate::ui::capture::{self, CaptureTarget};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Window, div, px};
use gpui_kit::component::{WindowExt, button::ButtonVariant};

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;
    let config = state.runtime.config();
    let macros = &config.macros;
    let selected = state
        .page
        .selected_macro
        .as_deref()
        .and_then(|name| macros.iter().position(|m| m.name == name))
        .or(if macros.is_empty() { None } else { Some(0) });
    let wide = window.viewport_size().width >= px(720.0);

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabMacros),
            tr(language, TextKey::MacrosSubtitle),
        ))
        .child(
            div()
                .flex()
                .when(wide, |node| node.flex_row().items_start().gap(px(16.0)))
                .when(!wide, |node| node.flex_col().gap(px(10.0)))
                .child(
                    div()
                        .when(wide, |node| node.w(px(280.0)).flex_none())
                        .when(!wide, |node| node.w_full())
                        .child(macro_column(state, shared, t, language, macros, selected)),
                )
                .child(
                    div()
                        .when(wide, |node| node.flex_1().min_w(px(420.0)))
                        .when(!wide, |node| node.w_full())
                        .child(editor_column(
                            state, shared, window, t, language, &config, selected, &mut *cx,
                        )),
                ),
        )
}

// ---------------------------------------------------------------------------
// Left column: macro list and ordering buttons.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn macro_column(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    macros: &[MacroDefinition],
    selected: Option<usize>,
) -> impl IntoElement {
    let rows = macros
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            let is_selected = Some(index) == selected;
            let name = definition.name.clone();
            let row_shared = shared.clone();
            div()
                .id(format!("macro-row-{name}"))
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py(px(7.0))
                .rounded(px(style::RADIUS_CONTROL))
                .when(is_selected, |d| d.bg(t.accent_soft))
                .hover(move |d| if is_selected { d } else { d.bg(t.card_hover) })
                .cursor_pointer()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(13.0))
                        .text_color(if is_selected { t.text } else { t.muted })
                        .child(name.clone()),
                )
                .on_click(move |_, window, cx| {
                    window.blur(cx);
                    let mut shell = row_shared.borrow_mut();
                    shell.page.selected_macro = Some(name.clone());
                    reset_edit_state(&mut shell.page);
                })
        })
        .collect::<Vec<_>>();

    let add_shared = shared.clone();
    let add_runtime = state.runtime.clone();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(controls::icon_text_button(
            "new-macro",
            t,
            gpui_icons::LucideIcon::Plus,
            tr(language, TextKey::NewMacroButton),
            move |_, window, cx| {
                window.blur(cx);
                let mut macros = add_runtime.config().macros;
                let name = unique_macro_name(&macros, language);
                macros.push(MacroDefinition {
                    name: name.clone(),
                    steps: vec![MacroStep::Delay { ms: 100 }],
                });
                if commit_macros(&add_runtime, macros) {
                    let mut shell = add_shared.borrow_mut();
                    shell.page.selected_macro = Some(name);
                    reset_edit_state(&mut shell.page);
                }
            },
        ))
        .when(!macros.is_empty(), |column| {
            column.child(
                div()
                    .rounded(px(style::RADIUS_CARD))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.card)
                    .p(px(8.0))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(rows),
            )
        })
}

// ---------------------------------------------------------------------------
// Right column: editor for the selected macro.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn editor_column(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    t: Tokens,
    language: Language,
    config: &Config,
    selected: Option<usize>,
    cx: &mut App,
) -> AnyElement {
    let Some(index) = selected else {
        return controls::card(
            "macros-empty",
            t,
            controls::empty_state(
                t,
                tr(language, TextKey::EmptyMacrosTitle),
                tr(language, TextKey::EmptyMacrosBody),
                tr(language, TextKey::NewMacroButton),
                style::ACCENT,
            ),
        )
        .into_any_element();
    };
    let Some(definition) = config.macros.get(index) else {
        return div().into_any_element();
    };

    let del_shared = shared.clone();
    let del_runtime = state.runtime.clone();
    let target = definition.name.clone();

    let delete = controls::danger_button(
        "delete-macro",
        gpui_icons::LucideIcon::Trash2,
        tr(language, TextKey::DeleteMacroButton),
        move |_, window, cx| {
            let dialog_shared = del_shared.clone();
            let dialog_runtime = del_runtime.clone();
            let dialog_target = target.clone();
            window.open_alert_dialog(cx, move |alert, _, _| {
                let ok_shared = dialog_shared.clone();
                let ok_runtime = dialog_runtime.clone();
                let ok_target = dialog_target.clone();
                alert
                    .confirm()
                    .title(tr(language, TextKey::ConfirmDeleteMacroTitle))
                    .description(tr(language, TextKey::ConfirmDeleteMacroBody))
                    .ok_text(tr(language, TextKey::ConfirmDelete))
                    .cancel_text(tr(language, TextKey::CancelAction))
                    .ok_variant(ButtonVariant::Danger)
                    .on_ok(move |_, _, _| {
                        let mut macros = ok_runtime.config().macros;
                        let Some(position) = macros.iter().position(|m| m.name == ok_target) else {
                            return true;
                        };
                        macros.remove(position);
                        let next = if macros.is_empty() {
                            None
                        } else {
                            Some(macros[position.min(macros.len() - 1)].name.clone())
                        };
                        if commit_macros(&ok_runtime, macros) {
                            let mut shell = ok_shared.borrow_mut();
                            shell.page.selected_macro = next;
                            reset_edit_state(&mut shell.page);
                        }
                        true
                    })
            });
        },
    )
    .into_any_element();

    controls::card(
        "macro-editor",
        t,
        editor(
            state, shared, window, t, language, config, definition, delete, cx,
        ),
    )
    .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn editor(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    t: Tokens,
    language: Language,
    _config: &Config,
    definition: &MacroDefinition,
    delete: AnyElement,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let name_field = controls::field_input(
        "macro-name",
        tr(language, TextKey::MacroNameLabel),
        definition.name.as_str(),
        Some(240.0),
        window,
        cx,
    );

    let steps = &definition.steps;
    let step_rows = steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            step_row(
                state,
                shared,
                window,
                t,
                language,
                &definition.name,
                index,
                step,
                steps.len(),
                cx,
            )
        })
        .collect::<Vec<_>>();

    let add_shared = shared.clone();
    let add_runtime = state.runtime.clone();
    let add_rendered = definition.name.clone();
    let add_buttons = StepKind::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            let kind = *kind;
            let button_shared = add_shared.clone();
            let button_runtime = add_runtime.clone();
            let rendered = add_rendered.clone();
            controls::icon_text_button(
                format!("add-step-{index}"),
                t,
                kind.icon(),
                kind.text(language),
                move |_, _, _| {
                    let name = {
                        let shell = button_shared.borrow();
                        shell
                            .page
                            .selected_macro
                            .clone()
                            .unwrap_or_else(|| rendered.clone())
                    };
                    let step = kind.default_step();
                    with_macro(&button_runtime, &name, |m| {
                        m.steps.push(step);
                        true
                    });
                    button_shared.borrow_mut().page.open_menu = None;
                },
            )
        })
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div().flex().flex_col().gap_2().child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(t.muted)
                            .child(tr(language, TextKey::MacroNameLabel)),
                    )
                    .child(name_field)
                    .child(div().flex_1().min_w(px(0.0)))
                    .child(delete),
            ),
        )
        .child(super::separator(t))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(14.0))
                        .text_color(t.text)
                        .child(tr(language, TextKey::MacroStepsTitle)),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(t.muted)
                        .child(steps.len().to_string()),
                ),
        )
        .child(if steps.is_empty() {
            div()
                .text_size(px(12.5))
                .text_color(t.muted)
                .child(tr(language, TextKey::EmptyStepsLabel))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .children(step_rows)
                .into_any_element()
        })
        .child(super::separator(t))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(13.0))
                        .text_color(t.text)
                        .child(tr(language, TextKey::AddStepButton)),
                )
                .child(div().flex().flex_wrap().gap_2())
                .children(add_buttons),
        )
}

// ---------------------------------------------------------------------------
// One macro step: kind picker, per-kind editors and row actions.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn step_row(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    t: Tokens,
    language: Language,
    macro_name: &str,
    index: usize,
    step: &MacroStep,
    step_count: usize,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let kind = StepKind::of(step);
    let kind_open = state.page.open_menu.as_deref() == Some("step-kind")
        && state.page.selected_step == Some(index);

    let kind_options = StepKind::ALL
        .iter()
        .map(|k| k.text(language).to_owned())
        .collect::<Vec<_>>();
    let kind_selected = StepKind::ALL.iter().position(|k| *k == kind).unwrap_or(0);

    let kind_shared = shared.clone();
    let kind_button = controls::select_button(
        "step-kind",
        t,
        kind.text(language),
        kind_open,
        150.0,
        move |_, window, cx| {
            window.blur(cx);
            let mut shell = kind_shared.borrow_mut();
            shell.page.selected_step = Some(index);
            shell.page.open_menu = if shell.page.open_menu.as_deref() == Some("step-kind") {
                None
            } else {
                Some("step-kind".to_owned())
            };
        },
    );
    let pick_shared = shared.clone();
    let pick_runtime = state.runtime.clone();
    let pick_rendered = macro_name.to_owned();
    let kind_menu = controls::select_menu(
        "step-kind",
        t,
        &kind_options,
        kind_selected,
        150.0,
        move |pick, _, _| {
            let picked = StepKind::ALL[pick];
            let mut shell = pick_shared.borrow_mut();
            shell.page.open_menu = None;
            shell.page.selected_step = Some(index);
            let name = shell
                .page
                .selected_macro
                .clone()
                .unwrap_or_else(|| pick_rendered.clone());
            drop(shell);
            with_macro(&pick_runtime, &name, |m| {
                if let Some(step) = m.steps.get_mut(index) {
                    let next = convert_step(step, picked);
                    *step = next;
                    true
                } else {
                    false
                }
            });
        },
    );

    let mut editors: Vec<AnyElement> = Vec::new();
    match step {
        MacroStep::Tap { chord } | MacroStep::Press { chord } | MacroStep::Release { chord } => {
            let target = CaptureTarget::MacroStep {
                name: macro_name.to_owned(),
                index,
            };
            let (capturing, label) =
                capture::chip(state, &target, *chord, tr(language, TextKey::PressKeyHint));
            let key_shared = shared.clone();
            let confirm_shared = shared.clone();
            let cancel_shared = shared.clone();
            let mut key_row =
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(controls::key_capture_button(
                        format!("step-{index}-key"),
                        t,
                        &label,
                        capturing,
                        move |_, window, cx| {
                            capture::start(window, cx, &key_shared, target.clone());
                        },
                    ));
            if capturing && state.capture_pending.is_some() {
                key_row = key_row.child(controls::capture_buttons(
                    &format!("step-{index}"),
                    t,
                    move |_, _, _| capture::confirm(&confirm_shared),
                    move |_, _, _| capture::cancel(&cancel_shared),
                ));
            }
            editors.push(key_row.into_any_element());
        }
        MacroStep::Text { text } => {
            editors.push(text_editor(window, language, index, text, cx));
        }
        MacroStep::Delay { ms } => {
            editors.push(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(t.muted)
                            .child(tr(language, TextKey::DelayMsLabel)),
                    )
                    .child(number_editor(
                        state,
                        window,
                        macro_name,
                        index,
                        *ms as i64,
                        0..=60_000,
                        cx,
                    ))
                    .into_any_element(),
            );
        }
        MacroStep::Scroll { axis, amount } => {
            let axis_options = vec![
                axis_label(language, ScrollAxis::Vertical).to_owned(),
                axis_label(language, ScrollAxis::Horizontal).to_owned(),
            ];
            let axis_selected = match axis {
                ScrollAxis::Vertical => 0,
                ScrollAxis::Horizontal => 1,
            };
            let axis_open = state.page.open_menu.as_deref() == Some("step-axis")
                && state.page.selected_step == Some(index);
            let axis_shared = shared.clone();
            let axis_button = controls::select_button(
                "step-axis",
                t,
                axis_label(language, *axis),
                axis_open,
                110.0,
                move |_, window, cx| {
                    window.blur(cx);
                    let mut shell = axis_shared.borrow_mut();
                    shell.page.selected_step = Some(index);
                    shell.page.open_menu = if shell.page.open_menu.as_deref() == Some("step-axis") {
                        None
                    } else {
                        Some("step-axis".to_owned())
                    };
                },
            );
            let axis_pick_shared = shared.clone();
            let axis_pick_runtime = state.runtime.clone();
            let axis_pick_rendered = macro_name.to_owned();
            let axis_menu = controls::select_menu(
                "step-axis",
                t,
                &axis_options,
                axis_selected,
                110.0,
                move |pick, _, _| {
                    let picked = if pick == 0 {
                        ScrollAxis::Vertical
                    } else {
                        ScrollAxis::Horizontal
                    };
                    let mut shell = axis_pick_shared.borrow_mut();
                    shell.page.open_menu = None;
                    shell.page.selected_step = Some(index);
                    let name = shell
                        .page
                        .selected_macro
                        .clone()
                        .unwrap_or_else(|| axis_pick_rendered.clone());
                    drop(shell);
                    with_macro(&axis_pick_runtime, &name, |m| {
                        if let Some(MacroStep::Scroll { axis, .. }) = m.steps.get_mut(index) {
                            *axis = picked;
                            true
                        } else {
                            false
                        }
                    });
                },
            );
            editors.push(
                div()
                    .relative()
                    .child(axis_button)
                    .when(axis_open, |d| d.child(axis_menu))
                    .into_any_element(),
            );
            editors.push(number_editor(
                state,
                window,
                macro_name,
                index,
                *amount as i64,
                -100..=100,
                cx,
            ));
        }
        MacroStep::MouseClick { button } => {
            let button_options = [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
                .iter()
                .map(|option| option.label(language).to_owned())
                .collect::<Vec<_>>();
            let button_selected = match button {
                MouseButton::Left => 0,
                MouseButton::Right => 1,
                MouseButton::Middle => 2,
            };
            let button_open = state.page.open_menu.as_deref() == Some("step-button")
                && state.page.selected_step == Some(index);
            let button_shared = shared.clone();
            let select_button = controls::select_button(
                "step-button",
                t,
                button.label(language),
                button_open,
                120.0,
                move |_, window, cx| {
                    window.blur(cx);
                    let mut shell = button_shared.borrow_mut();
                    shell.page.selected_step = Some(index);
                    shell.page.open_menu = if shell.page.open_menu.as_deref() == Some("step-button")
                    {
                        None
                    } else {
                        Some("step-button".to_owned())
                    };
                },
            );
            let pick_shared = shared.clone();
            let pick_runtime = state.runtime.clone();
            let pick_rendered = macro_name.to_owned();
            let button_menu = controls::select_menu(
                "step-button",
                t,
                &button_options,
                button_selected,
                120.0,
                move |pick, _, _| {
                    let picked = match pick {
                        0 => MouseButton::Left,
                        1 => MouseButton::Right,
                        _ => MouseButton::Middle,
                    };
                    let mut shell = pick_shared.borrow_mut();
                    shell.page.open_menu = None;
                    shell.page.selected_step = Some(index);
                    let name = shell
                        .page
                        .selected_macro
                        .clone()
                        .unwrap_or_else(|| pick_rendered.clone());
                    drop(shell);
                    with_macro(&pick_runtime, &name, |m| {
                        if let Some(MacroStep::MouseClick { button }) = m.steps.get_mut(index) {
                            *button = picked;
                            true
                        } else {
                            false
                        }
                    });
                },
            );
            editors.push(
                div()
                    .relative()
                    .child(select_button)
                    .when(button_open, |d| d.child(button_menu))
                    .into_any_element(),
            );
        }
    }

    let up_shared = shared.clone();
    let up_runtime = state.runtime.clone();
    let up_rendered = macro_name.to_owned();
    let up_button = controls::icon_button(
        format!("step-{index}-up"),
        t,
        gpui_icons::LucideIcon::ArrowUp,
        tr(language, TextKey::MoveStepUpButton),
        move |_, _, _| {
            let name = {
                let shell = up_shared.borrow();
                shell
                    .page
                    .selected_macro
                    .clone()
                    .unwrap_or_else(|| up_rendered.clone())
            };
            let moved = with_macro(&up_runtime, &name, |m| {
                if index > 0 && index < m.steps.len() {
                    m.steps.swap(index, index - 1);
                    true
                } else {
                    false
                }
            });
            if moved {
                let mut shell = up_shared.borrow_mut();
                shell.page.open_menu = None;
                match shell.page.selected_step {
                    Some(step) if step == index => shell.page.selected_step = Some(index - 1),
                    Some(step) if step == index - 1 => shell.page.selected_step = Some(index),
                    _ => {}
                }
            }
        },
    );

    let down_shared = shared.clone();
    let down_runtime = state.runtime.clone();
    let down_rendered = macro_name.to_owned();
    let down_button = controls::icon_button(
        format!("step-{index}-down"),
        t,
        gpui_icons::LucideIcon::ArrowDown,
        tr(language, TextKey::MoveStepDownButton),
        move |_, _, _| {
            let name = {
                let shell = down_shared.borrow();
                shell
                    .page
                    .selected_macro
                    .clone()
                    .unwrap_or_else(|| down_rendered.clone())
            };
            let moved = with_macro(&down_runtime, &name, |m| {
                if index + 1 < m.steps.len() {
                    m.steps.swap(index, index + 1);
                    true
                } else {
                    false
                }
            });
            if moved {
                let mut shell = down_shared.borrow_mut();
                shell.page.open_menu = None;
                match shell.page.selected_step {
                    Some(step) if step == index => shell.page.selected_step = Some(index + 1),
                    Some(step) if step == index + 1 => shell.page.selected_step = Some(index),
                    _ => {}
                }
            }
        },
    );

    let remove_shared = shared.clone();
    let remove_runtime = state.runtime.clone();
    let remove_rendered = macro_name.to_owned();
    let remove_button = controls::icon_button(
        format!("step-{index}-remove"),
        t,
        gpui_icons::LucideIcon::Trash2,
        tr(language, TextKey::RemoveStepButton),
        move |_, window, cx| {
            let dialog_shared = remove_shared.clone();
            let dialog_runtime = remove_runtime.clone();
            let dialog_rendered = remove_rendered.clone();
            window.open_alert_dialog(cx, move |alert, _, _| {
                let ok_shared = dialog_shared.clone();
                let ok_runtime = dialog_runtime.clone();
                let ok_rendered = dialog_rendered.clone();
                alert
                    .confirm()
                    .title(tr(language, TextKey::ConfirmDeleteStepTitle))
                    .description(tr(language, TextKey::ConfirmDeleteStepBody))
                    .ok_text(tr(language, TextKey::ConfirmDelete))
                    .cancel_text(tr(language, TextKey::CancelAction))
                    .ok_variant(ButtonVariant::Danger)
                    .on_ok(move |_, _, _| {
                        let name = {
                            let shell = ok_shared.borrow();
                            shell
                                .page
                                .selected_macro
                                .clone()
                                .unwrap_or_else(|| ok_rendered.clone())
                        };
                        let removed = with_macro(&ok_runtime, &name, |m| {
                            if index < m.steps.len() {
                                m.steps.remove(index);
                                true
                            } else {
                                false
                            }
                        });
                        if removed {
                            let mut shell = ok_shared.borrow_mut();
                            shell.page.open_menu = None;
                            if let Some(step) = shell.page.selected_step {
                                shell.page.selected_step = if step == index {
                                    None
                                } else if step > index {
                                    Some(step - 1)
                                } else {
                                    Some(step)
                                };
                            }
                        }
                        true
                    })
            });
        },
    );

    div()
        .id(format!("step-row-{index}"))
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py(px(7.0))
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(if state.page.selected_step == Some(index) {
            style::ACCENT
        } else {
            t.border
        })
        .child(
            div()
                .w(px(20.0))
                .text_size(px(11.0))
                .text_color(t.muted)
                .child(format!("{:02}", index + 1)),
        )
        .child(
            gpui_icons::lucide(kind.icon())
                .size(px(16.0))
                .text_color(t.muted),
        )
        .child(
            div()
                .relative()
                .child(kind_button)
                .when(kind_open, |d| d.child(kind_menu)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .children(editors),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .when(index > 0, |d| d.child(up_button))
                .when(index + 1 < step_count, |d| d.child(down_button))
                .child(remove_button),
        )
}

// ---------------------------------------------------------------------------
// Shared field editors (one focus handle per field kind across all rows).
// ---------------------------------------------------------------------------

fn number_editor(
    state: &ShellState,
    window: &mut Window,
    macro_name: &str,
    row: usize,
    value: i64,
    range: std::ops::RangeInclusive<i64>,
    cx: &mut App,
) -> AnyElement {
    // Each step owns an input keyed by its row, so caret and commit need no
    // shared-buffer bookkeeping anymore.
    let lo = *range.start();
    let hi = *range.end();
    let delta_runtime = state.runtime.clone();
    let rendered = macro_name.to_owned();

    controls::number_field(
        &format!("step-{row}-value"),
        value.to_string(),
        move |delta, _, _| {
            let next = (value + delta).clamp(lo, hi);
            with_macro(&delta_runtime, &rendered, |m| {
                if let Some(step) = m.steps.get_mut(row) {
                    set_numeric(step, next);
                    true
                } else {
                    false
                }
            });
        },
        window,
        cx,
    )
}

fn text_editor(
    window: &mut Window,
    language: Language,
    row: usize,
    text: &str,
    cx: &mut App,
) -> AnyElement {
    controls::field_input(
        &format!("step-{row}-text"),
        tr(language, TextKey::TextLabel),
        text,
        None,
        window,
        cx,
    )
}

// ---------------------------------------------------------------------------
// Pure helpers (mirrored from the eframe page; its tests live there until F4).
// ---------------------------------------------------------------------------

/// Clears step/menu/field-arm state after the edited macro changes.
fn reset_edit_state(page: &mut super::PageState) {
    page.selected_step = None;
    page.open_menu = None;
    page.macro_name_fresh = true;
    page.macro_text_fresh = true;
    page.step_value_fresh = true;
}

/// Applies `mutate` to the named macro and commits; returns whether the
/// configuration was written. Validation failures are reported.
pub(crate) fn with_macro(
    runtime: &RuntimeHandle,
    name: &str,
    mutate: impl FnOnce(&mut MacroDefinition) -> bool,
) -> bool {
    let mut macros = runtime.config().macros;
    let Some(definition) = macros.iter_mut().find(|m| m.name == name) else {
        return false;
    };
    if !mutate(definition) {
        return false;
    }
    commit_macros(runtime, macros)
}

/// Stores a key captured in the settings window in one key step of a macro.
pub(crate) fn capture_chord(runtime: &RuntimeHandle, name: &str, index: usize, captured: KeyChord) {
    with_macro(runtime, name, |m| {
        if let Some(chord) = m.steps.get_mut(index).and_then(chord_mut) {
            *chord = captured;
            true
        } else {
            false
        }
    });
}

/// Writes a full macro list back into the configuration.
fn commit_macros(runtime: &RuntimeHandle, macros: Vec<MacroDefinition>) -> bool {
    match runtime.update_config(move |config| config.macros = macros) {
        Ok(()) => true,
        Err(error) => {
            runtime.report_error(error.to_string());
            false
        }
    }
}

/// Renames `old` to `new`, tracking the selection on success.
pub(crate) fn rename_macro(runtime: &RuntimeHandle, shared: &Shared, old: &str, new: &str) {
    if old == new {
        return;
    }
    let mut macros = runtime.config().macros;
    let Some(definition) = macros.iter_mut().find(|m| m.name == old) else {
        return;
    };
    definition.name = new.to_owned();
    match runtime.update_config(move |config| config.macros = macros) {
        Ok(()) => {
            shared.borrow_mut().page.selected_macro = Some(new.to_owned());
        }
        Err(error) => runtime.report_error(error.to_string()),
    }
}

/// Writes a numeric step value, clamped to the old DragValue ranges.
pub(crate) fn set_numeric(step: &mut MacroStep, value: i64) {
    match step {
        MacroStep::Delay { ms } => *ms = value.clamp(0, 60_000) as u64,
        MacroStep::Scroll { amount, .. } => *amount = value.clamp(-100, 100) as i32,
        _ => {}
    }
}

/// Changes a step's kind, carrying over a compatible payload.
fn convert_step(step: &MacroStep, kind: StepKind) -> MacroStep {
    let mut new = kind.default_step();
    match (&mut new, step) {
        (
            MacroStep::Tap { chord } | MacroStep::Press { chord } | MacroStep::Release { chord },
            MacroStep::Tap { chord: previous }
            | MacroStep::Press { chord: previous }
            | MacroStep::Release { chord: previous },
        ) => *chord = *previous,
        (MacroStep::Text { text }, MacroStep::Text { text: previous }) => {
            *text = previous.clone();
        }
        (MacroStep::Delay { ms }, MacroStep::Delay { ms: previous }) => *ms = *previous,
        (
            MacroStep::Scroll { axis, amount },
            MacroStep::Scroll {
                axis: previous_axis,
                amount: previous_amount,
            },
        ) => {
            *axis = *previous_axis;
            *amount = *previous_amount;
        }
        (MacroStep::MouseClick { button }, MacroStep::MouseClick { button: previous }) => {
            *button = *previous;
        }
        _ => {}
    }
    new
}

/// Mutable chord of a key step, if the step holds one.
fn chord_mut(step: &mut MacroStep) -> Option<&mut KeyChord> {
    match step {
        MacroStep::Tap { chord } | MacroStep::Press { chord } | MacroStep::Release { chord } => {
            Some(chord)
        }
        _ => None,
    }
}

/// First free `"<TabMacros> <n>"` name, as in the eframe page.
fn unique_macro_name(macros: &[MacroDefinition], language: Language) -> String {
    let base = tr(language, TextKey::TabMacros);
    let mut number = 1;
    loop {
        let candidate = format!("{base} {number}");
        if !macros.iter().any(|definition| definition.name == candidate) {
            return candidate;
        }
        number += 1;
    }
}

fn axis_label(language: Language, axis: ScrollAxis) -> &'static str {
    match axis {
        ScrollAxis::Vertical => tr(language, TextKey::AxisVertical),
        ScrollAxis::Horizontal => tr(language, TextKey::AxisHorizontal),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepKind {
    Tap,
    Press,
    Release,
    Text,
    Delay,
    Scroll,
    MouseClick,
}

impl StepKind {
    const ALL: [Self; 7] = [
        Self::Tap,
        Self::Press,
        Self::Release,
        Self::Text,
        Self::Delay,
        Self::Scroll,
        Self::MouseClick,
    ];

    fn of(step: &MacroStep) -> Self {
        match step {
            MacroStep::Tap { .. } => Self::Tap,
            MacroStep::Press { .. } => Self::Press,
            MacroStep::Release { .. } => Self::Release,
            MacroStep::Text { .. } => Self::Text,
            MacroStep::Delay { .. } => Self::Delay,
            MacroStep::Scroll { .. } => Self::Scroll,
            MacroStep::MouseClick { .. } => Self::MouseClick,
        }
    }

    fn icon(self) -> gpui_icons::LucideIcon {
        match self {
            Self::Tap => gpui_icons::LucideIcon::MousePointerClick,
            Self::Press => gpui_icons::LucideIcon::ArrowDown,
            Self::Release => gpui_icons::LucideIcon::ArrowUp,
            Self::Text => gpui_icons::LucideIcon::Type,
            Self::Delay => gpui_icons::LucideIcon::Clock,
            Self::Scroll => gpui_icons::LucideIcon::MoveVertical,
            Self::MouseClick => gpui_icons::LucideIcon::Mouse,
        }
    }

    fn text(self, language: Language) -> &'static str {
        let key = match self {
            Self::Tap => TextKey::StepTap,
            Self::Press => TextKey::StepPress,
            Self::Release => TextKey::StepRelease,
            Self::Text => TextKey::StepText,
            Self::Delay => TextKey::StepDelay,
            Self::Scroll => TextKey::StepScroll,
            Self::MouseClick => TextKey::StepMouseClick,
        };
        tr(language, key)
    }

    fn default_step(self) -> MacroStep {
        let chord = KeyChord::new(KeyCode::KeyA);
        match self {
            Self::Tap => MacroStep::Tap { chord },
            Self::Press => MacroStep::Press { chord },
            Self::Release => MacroStep::Release { chord },
            Self::Text => MacroStep::Text {
                text: "…".to_owned(),
            },
            Self::Delay => MacroStep::Delay { ms: 100 },
            Self::Scroll => MacroStep::Scroll {
                axis: ScrollAxis::Vertical,
                amount: -3,
            },
            Self::MouseClick => MacroStep::MouseClick {
                button: MouseButton::Left,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_names_do_not_collide() {
        let existing = vec![
            MacroDefinition {
                name: "Macros 1".to_owned(),
                steps: vec![MacroStep::Delay { ms: 0 }],
            },
            MacroDefinition {
                name: "Macros 2".to_owned(),
                steps: vec![MacroStep::Delay { ms: 0 }],
            },
        ];

        let name = unique_macro_name(&existing, Language::En);
        assert_eq!(name, "Macros 3");
    }

    #[test]
    fn default_steps_are_valid_and_map_back() {
        for kind in StepKind::ALL {
            let step = kind.default_step();
            assert_eq!(StepKind::of(&step), kind);
            let definition = MacroDefinition {
                name: "macro".to_owned(),
                steps: vec![step],
            };
            assert_eq!(
                definition.validate(),
                Ok(()),
                "invalid default for {kind:?}"
            );
        }
    }
}
