//! Macros page: list and edit reusable input sequences (gpui port).

use crate::app::{AppError, RuntimeHandle};
use crate::core::action::{MouseButton, ScrollAxis};
use crate::core::{Config, KeyChord, KeyCode, Language, MacroDefinition, MacroStep, TextKey, tr};
use crate::ui::controls::{self, Edit};
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, App, ClickEvent, Context, Window, div, px};

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    _cx: &mut Context<SettingsView>,
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

    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabMacros),
            tr(language, TextKey::MacrosSubtitle),
        ))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(20.0))
                .child(macro_column(state, shared, t, language, macros, selected))
                .child(div().flex_1().min_w(px(0.0)).child(editor_column(
                    state, shared, window, t, language, &config, selected,
                ))),
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

    let delete_target = selected
        .and_then(|index| macros.get(index))
        .map(|definition| definition.name.clone());

    let add_shared = shared.clone();
    let add_runtime = state.runtime.clone();
    let del_shared = shared.clone();
    let del_runtime = state.runtime.clone();

    div()
        .flex_none()
        .w(px(280.0))
        .flex()
        .flex_col()
        .gap_3()
        .child(icon_text_button(
            t,
            "new-macro".to_owned(),
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
        .when_some(delete_target, |column, target| {
            column.child(icon_text_button(
                t,
                "delete-macro".to_owned(),
                gpui_icons::LucideIcon::Trash2,
                tr(language, TextKey::DeleteMacroButton),
                move |_, window, cx| {
                    window.blur(cx);
                    let mut macros = del_runtime.config().macros;
                    let Some(position) = macros.iter().position(|m| m.name == target) else {
                        return;
                    };
                    macros.remove(position);
                    let next = if macros.is_empty() {
                        None
                    } else {
                        Some(macros[position.min(macros.len() - 1)].name.clone())
                    };
                    if commit_macros(&del_runtime, macros) {
                        let mut shell = del_shared.borrow_mut();
                        shell.page.selected_macro = next;
                        reset_edit_state(&mut shell.page);
                    }
                },
            ))
        })
}

// ---------------------------------------------------------------------------
// Right column: editor for the selected macro.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn editor_column(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    language: Language,
    config: &Config,
    selected: Option<usize>,
) -> AnyElement {
    let Some(index) = selected else {
        return controls::card(
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
    controls::card(
        t,
        editor(state, shared, window, t, language, config, definition),
    )
    .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn editor(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    language: Language,
    config: &Config,
    definition: &MacroDefinition,
) -> impl IntoElement {
    // Name row: buffer shows while the field is focused and armed for
    // select-all-on-first-input; a pending invalid rename renders the same
    // warning the old page showed when `update_config` rejected it.
    let name_focused = state.page.focus_macro_name.is_focused(window);
    let name_shown = if name_focused && !state.page.macro_name_fresh {
        state.page.macro_name.as_str()
    } else {
        definition.name.as_str()
    };
    let name_warning = if name_focused && !state.page.macro_name_fresh {
        let mut candidate = config.clone();
        let pending = state.page.macro_name.clone();
        if let Some(m) = candidate
            .macros
            .iter_mut()
            .find(|m| m.name == definition.name)
        {
            m.name = pending;
        }
        candidate
            .validate()
            .err()
            .map(|error| AppError::Config(error).to_string())
    } else {
        None
    };

    let name_shared = shared.clone();
    let name_runtime = state.runtime.clone();
    let rendered = definition.name.clone();
    let name_field = controls::text_field(
        controls::Field {
            id: "macro-name",
            text: name_shown.to_owned(),
            placeholder: tr(language, TextKey::MacroNameLabel).to_owned(),
            focus: state.page.focus_macro_name.clone(),
            focused: name_focused,
            width: Some(240.0),
            tokens: t,
        },
        move |edit, _, _| {
            let is_focus = matches!(edit, Edit::Focused);
            let mut borrow = name_shared.borrow_mut();
            let shell = &mut *borrow;
            if is_focus {
                shell.page.open_menu = None;
            }
            controls::apply_edit(
                &mut shell.page.macro_name,
                &mut shell.page.macro_name_fresh,
                edit,
            );
            let buffer = shell.page.macro_name.clone();
            let target = shell
                .page
                .selected_macro
                .clone()
                .unwrap_or_else(|| rendered.clone());
            drop(borrow);
            if !is_focus {
                rename_macro(&name_runtime, &name_shared, &target, &buffer);
            }
        },
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
            icon_text_button(
                t,
                format!("add-step-{index}"),
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
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
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
                        .child(name_field),
                )
                .when_some(name_warning, |block, message| {
                    block.child(super::warning(&message))
                }),
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
    window: &Window,
    t: Tokens,
    language: Language,
    macro_name: &str,
    index: usize,
    step: &MacroStep,
    step_count: usize,
) -> impl IntoElement {
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
            let key_options = KeyCode::ALL
                .iter()
                .map(|key| key.as_str().to_owned())
                .collect::<Vec<_>>();
            let key_selected = KeyCode::ALL
                .iter()
                .position(|key| *key == chord.key)
                .unwrap_or(0);
            let key_open = state.page.open_menu.as_deref() == Some("step-key")
                && state.page.selected_step == Some(index);

            let key_shared = shared.clone();
            let key_button = controls::select_button(
                "step-key",
                t,
                chord.key.as_str(),
                key_open,
                92.0,
                move |_, window, cx| {
                    window.blur(cx);
                    let mut shell = key_shared.borrow_mut();
                    shell.page.selected_step = Some(index);
                    shell.page.open_menu = if shell.page.open_menu.as_deref() == Some("step-key") {
                        None
                    } else {
                        Some("step-key".to_owned())
                    };
                },
            );
            let key_pick_shared = shared.clone();
            let key_pick_runtime = state.runtime.clone();
            let key_pick_rendered = macro_name.to_owned();
            let key_menu = controls::select_menu(
                "step-key",
                t,
                &key_options,
                key_selected,
                move |pick, _, _| {
                    let picked = KeyCode::ALL[pick];
                    let mut shell = key_pick_shared.borrow_mut();
                    shell.page.open_menu = None;
                    shell.page.selected_step = Some(index);
                    let name = shell
                        .page
                        .selected_macro
                        .clone()
                        .unwrap_or_else(|| key_pick_rendered.clone());
                    drop(shell);
                    with_macro(&key_pick_runtime, &name, |m| {
                        if let Some(chord) = m.steps.get_mut(index).and_then(chord_mut) {
                            chord.key = picked;
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
                    .child(key_button)
                    .when(key_open, |d| d.child(key_menu))
                    .into_any_element(),
            );

            for (mod_key, enabled) in [
                (TextKey::ModifierControl, chord.modifiers.control),
                (TextKey::ModifierAlt, chord.modifiers.alt),
                (TextKey::ModifierShift, chord.modifiers.shift),
                (TextKey::ModifierMeta, chord.modifiers.meta),
            ] {
                let id = format!("step-{index}-mod-{mod_key:?}");
                let label = tr(language, mod_key);
                let wanted = !enabled;
                let mod_shared = shared.clone();
                let mod_runtime = state.runtime.clone();
                let mod_rendered = macro_name.to_owned();
                editors.push(
                    controls::toggle_labeled(id, t, label, enabled, move |_, _, _| {
                        let name = {
                            let shell = mod_shared.borrow();
                            shell
                                .page
                                .selected_macro
                                .clone()
                                .unwrap_or_else(|| mod_rendered.clone())
                        };
                        with_macro(&mod_runtime, &name, |m| {
                            if let Some(chord) = m.steps.get_mut(index).and_then(chord_mut) {
                                match mod_key {
                                    TextKey::ModifierControl => chord.modifiers.control = wanted,
                                    TextKey::ModifierAlt => chord.modifiers.alt = wanted,
                                    TextKey::ModifierShift => chord.modifiers.shift = wanted,
                                    TextKey::ModifierMeta => chord.modifiers.meta = wanted,
                                    _ => {}
                                }
                                true
                            } else {
                                false
                            }
                        });
                    })
                    .into_any_element(),
                );
            }
        }
        MacroStep::Text { text } => {
            editors.push(
                text_editor(state, shared, window, t, language, macro_name, index, text)
                    .into_any_element(),
            );
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
                        shared,
                        window,
                        t,
                        macro_name,
                        index,
                        (*ms).to_string(),
                        *ms as i64,
                        0..=60_000,
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
            editors.push(
                number_editor(
                    state,
                    shared,
                    window,
                    t,
                    macro_name,
                    index,
                    (*amount).to_string(),
                    *amount as i64,
                    -100..=100,
                )
                .into_any_element(),
            );
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
    let up_button = icon_button(
        t,
        format!("step-{index}-up"),
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
    let down_button = icon_button(
        t,
        format!("step-{index}-down"),
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
    let remove_button = icon_button(
        t,
        format!("step-{index}-remove"),
        gpui_icons::LucideIcon::Trash2,
        tr(language, TextKey::RemoveStepButton),
        move |_, _, _| {
            let name = {
                let shell = remove_shared.borrow();
                shell
                    .page
                    .selected_macro
                    .clone()
                    .unwrap_or_else(|| remove_rendered.clone())
            };
            let removed = with_macro(&remove_runtime, &name, |m| {
                if index < m.steps.len() {
                    m.steps.remove(index);
                    true
                } else {
                    false
                }
            });
            if removed {
                let mut shell = remove_shared.borrow_mut();
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

#[allow(clippy::too_many_arguments)]
fn number_editor(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    macro_name: &str,
    row: usize,
    current: String,
    value: i64,
    range: std::ops::RangeInclusive<i64>,
) -> impl IntoElement {
    // The focus handle is shared by every numeric field, so the buffer is only
    // shown for the row that owns it (`selected_step` is set on click).
    let focused =
        state.page.focus_step_value.is_focused(window) && state.page.selected_step == Some(row);
    let shown = if focused && !state.page.step_value_fresh {
        state.page.step_value.as_str()
    } else {
        current.as_str()
    };
    let field = controls::Field {
        id: "step-value",
        text: shown.to_owned(),
        placeholder: current.clone(),
        focus: state.page.focus_step_value.clone(),
        focused,
        width: Some(72.0),
        tokens: t,
    };
    let lo = *range.start();
    let hi = *range.end();
    let rendered = macro_name.to_owned();
    let edit_shared = shared.clone();
    let edit_runtime = state.runtime.clone();
    let delta_shared = shared.clone();
    let delta_runtime = state.runtime.clone();
    let delta_rendered = rendered.clone();

    controls::number_field(
        field,
        move |edit, _, _| {
            let is_focus = matches!(edit, Edit::Focused);
            let mut borrow = edit_shared.borrow_mut();
            let shell = &mut *borrow;
            if is_focus {
                shell.page.selected_step = Some(row);
                shell.page.open_menu = None;
            }
            controls::apply_edit(
                &mut shell.page.step_value,
                &mut shell.page.step_value_fresh,
                edit,
            );
            let buffer = shell.page.step_value.clone();
            let target_step = shell.page.selected_step;
            let target_macro = shell
                .page
                .selected_macro
                .clone()
                .unwrap_or_else(|| rendered.clone());
            drop(borrow);
            // Keystrokes may dispatch to any field sharing the focus handle, so
            // the commit target comes from `selected_step`, set on click.
            if !is_focus
                && let Some(index) = target_step
                && let Ok(parsed) = buffer.parse::<i64>()
            {
                with_macro(&edit_runtime, &target_macro, |m| {
                    if let Some(step) = m.steps.get_mut(index) {
                        set_numeric(step, parsed);
                        true
                    } else {
                        false
                    }
                });
            }
        },
        move |delta, _, _| {
            let next = (value + delta).clamp(lo, hi);
            let name = {
                let shell = delta_shared.borrow();
                shell
                    .page
                    .selected_macro
                    .clone()
                    .unwrap_or_else(|| delta_rendered.clone())
            };
            with_macro(&delta_runtime, &name, |m| {
                if let Some(step) = m.steps.get_mut(row) {
                    set_numeric(step, next);
                    true
                } else {
                    false
                }
            });
            let mut borrow = delta_shared.borrow_mut();
            let shell = &mut *borrow;
            shell.page.step_value = next.to_string();
            shell.page.step_value_fresh = true;
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn text_editor(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    language: Language,
    macro_name: &str,
    row: usize,
    text: &str,
) -> impl IntoElement {
    // Shared focus handle: only the selected row shows the buffer.
    let focused =
        state.page.focus_macro_text.is_focused(window) && state.page.selected_step == Some(row);
    let shown = if focused && !state.page.macro_text_fresh {
        state.page.macro_text.as_str()
    } else {
        text
    };
    let field = controls::Field {
        id: "macro-text",
        text: shown.to_owned(),
        placeholder: tr(language, TextKey::TextLabel).to_owned(),
        focus: state.page.focus_macro_text.clone(),
        focused,
        width: None,
        tokens: t,
    };
    let rendered = macro_name.to_owned();
    let edit_shared = shared.clone();
    let edit_runtime = state.runtime.clone();

    controls::text_field(field, move |edit, _, _| {
        let is_focus = matches!(edit, Edit::Focused);
        let mut borrow = edit_shared.borrow_mut();
        let shell = &mut *borrow;
        if is_focus {
            shell.page.selected_step = Some(row);
            shell.page.open_menu = None;
        }
        controls::apply_edit(
            &mut shell.page.macro_text,
            &mut shell.page.macro_text_fresh,
            edit,
        );
        let buffer = shell.page.macro_text.clone();
        let target_step = shell.page.selected_step;
        let target_macro = shell
            .page
            .selected_macro
            .clone()
            .unwrap_or_else(|| rendered.clone());
        drop(borrow);
        // Keystrokes may dispatch to any field sharing the focus handle, so the
        // commit target comes from `selected_step`, set on click.
        if !is_focus && let Some(index) = target_step {
            with_macro(&edit_runtime, &target_macro, |m| {
                if let Some(MacroStep::Text { text: slot }) = m.steps.get_mut(index) {
                    *slot = buffer;
                    true
                } else {
                    false
                }
            });
        }
    })
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
fn with_macro(
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
fn rename_macro(runtime: &RuntimeHandle, shared: &Shared, old: &str, new: &str) {
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
fn set_numeric(step: &mut MacroStep, value: i64) {
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

fn icon_text_button(
    t: Tokens,
    id: String,
    icon: gpui_icons::LucideIcon,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::CONTROL_HEIGHT))
        .px_3()
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(t.border)
        .text_size(px(13.5))
        .text_color(t.text)
        .cursor_pointer()
        .hover(|this| this.bg(t.card_hover))
        .child(gpui_icons::lucide(icon).size(px(16.0)).text_color(t.muted))
        .child(label.to_owned())
        .on_click(on_click)
}

fn icon_button(
    t: Tokens,
    id: String,
    icon: gpui_icons::LucideIcon,
    tooltip: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(30.0))
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(t.border)
        .cursor_pointer()
        .hover(|this| this.bg(t.card_hover))
        .tooltip(controls::text_tooltip(tooltip.to_owned()))
        .child(gpui_icons::lucide(icon).size(px(16.0)).text_color(t.text))
        .on_click(on_click)
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
