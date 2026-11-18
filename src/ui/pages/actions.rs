//! Actions page: manage the action sets shown in the wheel (gpui port).

use crate::app::RuntimeHandle;
use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::{
    Action, ActionSet, Config, Control, KeyChord, KeyCode, Language, SetIcon, TextKey,
    WheelGeometry, tr,
};
use crate::ui::controls::{self, Edit};
use crate::ui::iconset;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use crate::ui::wheel_paint;
use gpui::prelude::*;
use gpui::{AnyElement, App, ClickEvent, Context, Window, canvas, div, point, px};
use std::path::PathBuf;

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;
    let config = state.runtime.config();
    let sets = config.action_sets.clone();
    let active = config.active_set.clone();

    let selected = state
        .page
        .selected_set
        .as_deref()
        .and_then(|id| sets.iter().position(|set| set.id == id))
        .or(if sets.is_empty() { None } else { Some(0) });

    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabActions),
            tr(language, TextKey::ActionsSubtitle),
        ))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(20.0))
                .child(set_column(state, shared, cx, t, language, &sets, selected))
                .child(div().flex_1().min_w(px(0.0)).child(editor_column(
                    state, shared, window, t, language, &config, &sets, &active, selected,
                ))),
        )
}

// ---------------------------------------------------------------------------
// Left column: set list and ordering buttons.
// ---------------------------------------------------------------------------

fn set_column(
    state: &ShellState,
    shared: &Shared,
    cx: &mut Context<SettingsView>,
    t: Tokens,
    language: Language,
    sets: &[ActionSet],
    selected: Option<usize>,
) -> impl IntoElement {
    let list_shared = shared.clone();
    let rows = sets
        .iter()
        .enumerate()
        .map(|(index, set)| {
            let is_selected = Some(index) == selected;
            let is_active = state.runtime.config().active_set.as_deref() == Some(set.id.as_str());
            let id = set.id.clone();
            let row_shared = list_shared.clone();
            div()
                .id(format!("set-row-{}", set.id))
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py(px(7.0))
                .rounded(px(style::RADIUS_CONTROL))
                .when(is_selected, |div| div.bg(t.accent_soft))
                .hover(move |div| {
                    if is_selected {
                        div
                    } else {
                        div.bg(t.card_hover)
                    }
                })
                .cursor_pointer()
                .child(iconset::element(
                    set.icon,
                    18.0,
                    if is_selected { style::ACCENT } else { t.muted },
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_size(px(13.0))
                        .text_color(if is_selected { t.text } else { t.muted })
                        .child(set.name.clone()),
                )
                .when(is_active, |div| {
                    div.child(controls::inline_pill(
                        tr(language, TextKey::ActiveSetLabel),
                        style::SUCCESS,
                    ))
                })
                .on_click(move |_, _, _| {
                    row_shared.borrow_mut().page.selected_set = Some(id.clone());
                })
        })
        .collect::<Vec<_>>();

    let can_move_left = selected.is_some_and(|index| index > 0);
    let can_move_right = selected.is_some_and(|index| index + 1 < sets.len());

    let add_shared = shared.clone();
    let add_runtime = state.runtime.clone();
    let left_shared = shared.clone();
    let left_runtime = state.runtime.clone();
    let right_shared = shared.clone();
    let right_runtime = state.runtime.clone();

    let preview = preview_box(state, shared, cx, sets, selected);

    div()
        .flex_none()
        .w(px(320.0))
        .flex()
        .flex_col()
        .gap_3()
        .child(preview)
        .child(
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
        .when(sets.is_empty(), |section| {
            section.child(
                div()
                    .text_size(px(12.0))
                    .text_color(t.muted)
                    .child(tr(language, TextKey::EmptyActionsBody)),
            )
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(icon_text_button(
                    t,
                    "add-set",
                    gpui_icons::LucideIcon::Plus,
                    &format!("  {}", tr(language, TextKey::AddSetButton)),
                    move |_, _, _| {
                        let mut sets = add_runtime.config().action_sets.clone();
                        let set_language = add_runtime.config().language;
                        let set = new_set(&sets, set_language);
                        let id = set.id.clone();
                        sets.push(set);
                        commit_sets(&add_runtime, sets, add_runtime.config().active_set.clone());
                        add_shared.borrow_mut().page.selected_set = Some(id);
                    },
                ))
                .when(can_move_left, |div| {
                    div.child(icon_button(
                        t,
                        "move-set-left",
                        gpui_icons::LucideIcon::ChevronLeft,
                        tr(language, TextKey::MoveSetLeft),
                        move |_, _, _| {
                            if let Some(index) = move_sets(&left_runtime, &left_shared, -1) {
                                let mut state = left_shared.borrow_mut();
                                let sets = &state.runtime.config().action_sets;
                                if let Some(set) = sets.get(index) {
                                    state.page.selected_set = Some(set.id.clone());
                                }
                            }
                        },
                    ))
                })
                .when(can_move_right, |div| {
                    div.child(icon_button(
                        t,
                        "move-set-right",
                        gpui_icons::LucideIcon::ChevronRight,
                        tr(language, TextKey::MoveSetRight),
                        move |_, _, _| {
                            if let Some(index) = move_sets(&right_runtime, &right_shared, 1) {
                                let mut state = right_shared.borrow_mut();
                                let sets = &state.runtime.config().action_sets;
                                if let Some(set) = sets.get(index) {
                                    state.page.selected_set = Some(set.id.clone());
                                }
                            }
                        },
                    ))
                }),
        )
}

// ---------------------------------------------------------------------------
// Right column: editor for the selected set.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn editor_column(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    language: Language,
    config: &Config,
    sets: &[ActionSet],
    active: &Option<String>,
    selected: Option<usize>,
) -> impl IntoElement {
    if sets.is_empty() {
        return controls::card(
            t,
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(15.0))
                .text_color(t.text)
                .child(tr(language, TextKey::EmptyActionsTitle)),
        )
        .into_any_element();
    }

    let index = selected.unwrap_or(0);
    let Some(set) = sets.get(index) else {
        return div().into_any_element();
    };
    let is_active = active.as_deref() == Some(set.id.as_str());
    let brightness = state.platform.brightness_supported;

    let delete_shared = shared.clone();
    let delete_runtime = state.runtime.clone();
    let delete_id = set.id.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(controls::card(
            t,
            editor(
                state, shared, window, t, language, config, set, is_active, brightness,
            ),
        ))
        .child(super::separator(t))
        .child(icon_text_button(
            t,
            "delete-set",
            gpui_icons::LucideIcon::Trash,
            &format!("  {}", tr(language, TextKey::DeleteSetButton)),
            move |_, _, _| {
                let mut sets = delete_runtime.config().action_sets.clone();
                let mut active = delete_runtime.config().active_set.clone();
                let Some(position) = sets.iter().position(|set| set.id == delete_id) else {
                    return;
                };
                let removed = sets.remove(position);
                if active.as_deref() == Some(removed.id.as_str()) {
                    active = sets.first().map(|set| set.id.clone());
                }
                let next = if sets.is_empty() {
                    None
                } else {
                    Some(position.min(sets.len() - 1))
                };
                commit_sets(&delete_runtime, sets, active);
                delete_shared.borrow_mut().page.selected_set = next.and_then(|index| {
                    delete_runtime
                        .config()
                        .action_sets
                        .get(index)
                        .map(|set| set.id.clone())
                });
            },
        ))
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
    set: &ActionSet,
    is_active: bool,
    brightness_supported: bool,
) -> impl IntoElement {
    let set_id = set.id.clone();

    // Name row.
    let name_focused = state.page.focus_set_name.is_focused(window);
    let name_shown = if name_focused && !state.page.set_name_fresh {
        state.page.set_name.as_str()
    } else {
        set.name.as_str()
    };
    let name_buffer = shared.clone();
    let name_runtime = state.runtime.clone();
    let name_id = set_id.clone();
    let name_field = controls::text_field(
        controls::Field {
            id: "set-name",
            text: name_shown.to_owned(),
            placeholder: tr(language, TextKey::SetNameLabel).to_owned(),
            focus: state.page.focus_set_name.clone(),
            focused: name_focused,
            width: Some(220.0),
            tokens: t,
        },
        move |edit, _, _| {
            let commit = !matches!(edit, Edit::Focused);
            let mut borrow = name_buffer.borrow_mut();
            let shell = &mut *borrow;
            controls::apply_edit(
                &mut shell.page.set_name,
                &mut shell.page.set_name_fresh,
                edit,
            );
            let name = shell.page.set_name.clone();
            drop(borrow);
            if commit {
                with_set(&name_runtime, &name_id, move |set| set.name = name);
            }
        },
    );

    // Icon picker.
    let icon_rows = SetIcon::ALL
        .chunks(10)
        .map(|row| {
            div().flex().gap_1().children(row.iter().map(|icon| {
                let is_selected = set.icon == *icon;
                let icon_shared = shared.clone();
                let icon_runtime = state.runtime.clone();
                let icon_id = set_id.clone();
                let icon = *icon;
                div()
                    .id(format!("set-icon-{}", iconset::name(icon)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(34.0))
                    .rounded(px(7.0))
                    .when(is_selected, |div| {
                        div.bg(t.accent_soft).border_1().border_color(style::ACCENT)
                    })
                    .hover(move |div| {
                        if is_selected {
                            div
                        } else {
                            div.bg(t.card_hover)
                        }
                    })
                    .cursor_pointer()
                    .child(iconset::element(
                        icon,
                        22.0,
                        if is_selected { style::ACCENT } else { t.text },
                    ))
                    .on_click(move |_, _, _| {
                        let _ = &icon_shared;
                        with_set(&icon_runtime, &icon_id, move |set| set.icon = icon);
                    })
            }))
        })
        .collect::<Vec<_>>();

    let controls_column = [Control::Left, Control::Press, Control::Right]
        .iter()
        .map(|control| {
            show_control(
                state,
                shared,
                window,
                t,
                language,
                config,
                set,
                *control,
                brightness_supported,
            )
            .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(t.muted)
                        .child(tr(language, TextKey::SetNameLabel)),
                )
                .child(name_field)
                .when(is_active, |div| {
                    div.child(controls::status_pill(
                        tr(language, TextKey::ActiveSetLabel),
                        style::SUCCESS,
                    ))
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(t.muted)
                        .child(tr(language, TextKey::SetIconLabel)),
                )
                .children(icon_rows),
        )
        .children(controls_column)
}

// ---------------------------------------------------------------------------
// Control sections and parameters.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn show_control(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    language: Language,
    config: &Config,
    set: &ActionSet,
    control: Control,
    brightness_supported: bool,
) -> impl IntoElement {
    let ids = MenuIds::of(control);
    let kind = Kind::of(set.actions.for_control(control));
    let choices = Kind::choices(brightness_supported);
    let options = choices
        .iter()
        .map(|choice| choice.label(language).to_owned())
        .collect::<Vec<_>>();
    let selected = choices
        .iter()
        .position(|choice| *choice == kind)
        .unwrap_or(0);
    let open = state.page.open_menu.as_deref() == Some(ids.kind);

    let kind_shared = shared.clone();
    let pick_shared = shared.clone();
    let pick_runtime = state.runtime.clone();
    let pick_id = set.id.clone();

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
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(13.5))
                        .text_color(t.text)
                        .child(control_label(language, control)),
                )
                .child(
                    div()
                        .relative()
                        .child(controls::select_button(
                            ids.kind,
                            t,
                            kind.label(language),
                            open,
                            170.0,
                            move |_, _, _| {
                                let mut shell = kind_shared.borrow_mut();
                                shell.page.open_menu =
                                    if shell.page.open_menu.as_deref() == Some(ids.kind) {
                                        None
                                    } else {
                                        Some(ids.kind.to_owned())
                                    };
                            },
                        ))
                        .when(open, |div| {
                            div.child(controls::select_menu(
                                format!("{}-menu", ids.kind),
                                t,
                                &options,
                                selected,
                                move |index, _, _| {
                                    let choice = choices[index];
                                    with_set_action(
                                        &pick_runtime,
                                        &pick_id,
                                        control,
                                        |action, cfg| {
                                            *action = choice.default_action(cfg);
                                        },
                                    );
                                    pick_shared.borrow_mut().page.open_menu = None;
                                },
                            ))
                        }),
                ),
        )
        .child(
            div()
                .mt(px(4.0))
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .children(parameters(
                    state, shared, window, t, language, config, set, control, ids,
                )),
        )
}

#[allow(clippy::too_many_arguments)]
fn parameters(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    language: Language,
    config: &Config,
    set: &ActionSet,
    control: Control,
    ids: MenuIds,
) -> Vec<AnyElement> {
    let action = set.actions.for_control(control).clone();
    let mut out: Vec<AnyElement> = Vec::new();

    match action {
        Action::None => {
            out.push(muted(t, tr(language, TextKey::ActionNone)).into_any_element());
        }
        Action::Mute => {
            out.push(muted(t, tr(language, TextKey::ActionMute)).into_any_element());
        }
        Action::Volume { direction, steps } | Action::Brightness { direction, steps } => {
            out.push(muted(t, tr(language, TextKey::ActionDirection)).into_any_element());
            out.push(
                enum_select(
                    shared,
                    t,
                    ids.dir,
                    direction_label(language, direction),
                    state.page.open_menu.as_deref() == Some(ids.dir),
                    vec![
                        direction_label(language, VolumeDirection::Up).to_owned(),
                        direction_label(language, VolumeDirection::Down).to_owned(),
                    ],
                    match direction {
                        VolumeDirection::Up => 0,
                        VolumeDirection::Down => 1,
                    },
                    130.0,
                    {
                        let pick_state = shared.clone();
                        let runtime = state.runtime.clone();
                        let set_id = set.id.clone();
                        move |index, _, _| {
                            let direction = match index {
                                0 => VolumeDirection::Up,
                                _ => VolumeDirection::Down,
                            };
                            with_set_action(&runtime, &set_id, control, |action, _| {
                                if let Action::Volume {
                                    direction: slot, ..
                                }
                                | Action::Brightness {
                                    direction: slot, ..
                                } = action
                                {
                                    *slot = direction;
                                }
                            });
                            pick_state.borrow_mut().page.open_menu = None;
                        }
                    },
                )
                .into_any_element(),
            );
            out.push(muted(t, tr(language, TextKey::ActionSteps)).into_any_element());
            out.push(
                number_editor(
                    state,
                    shared,
                    window,
                    t,
                    ids.steps,
                    steps.to_string(),
                    i64::from(steps),
                    1..=10,
                    set.id.clone(),
                    control,
                )
                .into_any_element(),
            );
        }
        Action::Scroll { axis, amount } => {
            out.push(muted(t, tr(language, TextKey::ActionAxis)).into_any_element());
            out.push(
                enum_select(
                    shared,
                    t,
                    ids.axis,
                    axis_label(language, axis),
                    state.page.open_menu.as_deref() == Some(ids.axis),
                    vec![
                        axis_label(language, ScrollAxis::Vertical).to_owned(),
                        axis_label(language, ScrollAxis::Horizontal).to_owned(),
                    ],
                    match axis {
                        ScrollAxis::Vertical => 0,
                        ScrollAxis::Horizontal => 1,
                    },
                    130.0,
                    {
                        let pick_state = shared.clone();
                        let runtime = state.runtime.clone();
                        let set_id = set.id.clone();
                        move |index, _, _| {
                            let axis = match index {
                                0 => ScrollAxis::Vertical,
                                _ => ScrollAxis::Horizontal,
                            };
                            with_set_action(&runtime, &set_id, control, |action, _| {
                                if let Action::Scroll { axis: slot, .. } = action {
                                    *slot = axis;
                                }
                            });
                            pick_state.borrow_mut().page.open_menu = None;
                        }
                    },
                )
                .into_any_element(),
            );
            out.push(muted(t, tr(language, TextKey::ActionAmount)).into_any_element());
            out.push(
                number_editor(
                    state,
                    shared,
                    window,
                    t,
                    ids.amount,
                    amount.to_string(),
                    i64::from(amount),
                    -100..=100,
                    set.id.clone(),
                    control,
                )
                .into_any_element(),
            );
        }
        Action::Macro { name } => {
            out.push(muted(t, tr(language, TextKey::ActionMacroName)).into_any_element());
            let options = config
                .macros
                .iter()
                .map(|definition| definition.name.clone())
                .collect::<Vec<_>>();
            let selected = options.iter().position(|option| *option == name);
            out.push(
                enum_select(
                    shared,
                    t,
                    ids.macro_pick,
                    if name.is_empty() { "—" } else { &name },
                    state.page.open_menu.as_deref() == Some(ids.macro_pick),
                    options.clone(),
                    selected.unwrap_or(0),
                    200.0,
                    {
                        let pick_state = shared.clone();
                        let runtime = state.runtime.clone();
                        let set_id = set.id.clone();
                        move |index, _, _| {
                            let picked = options[index].clone();
                            with_set_action(&runtime, &set_id, control, |action, _| {
                                if let Action::Macro { name: slot } = action {
                                    *slot = picked;
                                }
                            });
                            pick_state.borrow_mut().page.open_menu = None;
                        }
                    },
                )
                .into_any_element(),
            );
        }
        Action::Shortcut { chord } => {
            out.push(muted(t, tr(language, TextKey::ActionChord)).into_any_element());
            out.push(
                chord_picker(
                    state,
                    shared,
                    t,
                    language,
                    &ids,
                    control,
                    set.id.clone(),
                    chord,
                )
                .into_any_element(),
            );
        }
        Action::MouseClick { button } => {
            out.push(muted(t, tr(language, TextKey::ActionButton)).into_any_element());
            let buttons = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
            let selected = buttons
                .iter()
                .position(|option| *option == button)
                .unwrap_or(0);
            out.push(
                enum_select(
                    shared,
                    t,
                    ids.mouse_btn,
                    button.label(language),
                    state.page.open_menu.as_deref() == Some(ids.mouse_btn),
                    buttons
                        .iter()
                        .map(|option| option.label(language).to_owned())
                        .collect(),
                    selected,
                    160.0,
                    {
                        let pick_state = shared.clone();
                        let runtime = state.runtime.clone();
                        let set_id = set.id.clone();
                        move |index, _, _| {
                            let picked = buttons[index];
                            with_set_action(&runtime, &set_id, control, |action, _| {
                                if let Action::MouseClick { button: slot } = action {
                                    *slot = picked;
                                }
                            });
                            pick_state.borrow_mut().page.open_menu = None;
                        }
                    },
                )
                .into_any_element(),
            );
        }
        Action::Media { command } => {
            out.push(muted(t, tr(language, TextKey::ActionCommand)).into_any_element());
            let commands = [
                MediaCommand::PlayPause,
                MediaCommand::Next,
                MediaCommand::Previous,
                MediaCommand::Stop,
            ];
            let selected = commands
                .iter()
                .position(|option| *option == command)
                .unwrap_or(0);
            out.push(
                enum_select(
                    shared,
                    t,
                    ids.media,
                    media_label(language, command),
                    state.page.open_menu.as_deref() == Some(ids.media),
                    commands
                        .iter()
                        .map(|option| media_label(language, *option).to_owned())
                        .collect(),
                    selected,
                    180.0,
                    {
                        let pick_state = shared.clone();
                        let runtime = state.runtime.clone();
                        let set_id = set.id.clone();
                        move |index, _, _| {
                            let picked = commands[index];
                            with_set_action(&runtime, &set_id, control, |action, _| {
                                if let Action::Media { command: slot } = action {
                                    *slot = picked;
                                }
                            });
                            pick_state.borrow_mut().page.open_menu = None;
                        }
                    },
                )
                .into_any_element(),
            );
        }
        Action::Launch { path } => {
            out.push(muted(t, tr(language, TextKey::ActionPath)).into_any_element());
            let focused = state.page.focus_path.is_focused(window);
            let current = path.to_string_lossy().into_owned();
            let shown = if focused && !state.page.path_buffer_fresh {
                state.page.path_buffer.as_str()
            } else {
                current.as_str()
            };
            let path_shared = shared.clone();
            let path_runtime = state.runtime.clone();
            let path_id = set.id.clone();
            out.push(
                controls::text_field(
                    controls::Field {
                        id: ids.path,
                        text: shown.to_owned(),
                        placeholder: "…".to_owned(),
                        focus: state.page.focus_path.clone(),
                        focused,
                        width: Some(320.0),
                        tokens: t,
                    },
                    move |edit, _, _| {
                        let commit = !matches!(edit, Edit::Focused);
                        let mut borrow = path_shared.borrow_mut();
                        let shell = &mut *borrow;
                        controls::apply_edit(
                            &mut shell.page.path_buffer,
                            &mut shell.page.path_buffer_fresh,
                            edit,
                        );
                        let buffer = shell.page.path_buffer.clone();
                        drop(borrow);
                        if commit {
                            with_set_action(&path_runtime, &path_id, control, |action, _| {
                                if let Action::Launch { path: slot } = action {
                                    *slot = PathBuf::from(&buffer);
                                }
                            });
                        }
                    },
                )
                .into_any_element(),
            );
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Shared building blocks.
// ---------------------------------------------------------------------------

fn muted(t: Tokens, text: &str) -> impl IntoElement {
    div()
        .text_size(px(12.5))
        .text_color(t.muted)
        .child(text.to_owned())
}

/// Dropdown for one parameter; the pick closure carries the concrete value.
#[allow(clippy::too_many_arguments)]
fn enum_select(
    shared: &Shared,
    t: Tokens,
    menu_id: &'static str,
    value: &str,
    open: bool,
    options: Vec<String>,
    selected: usize,
    width: f32,
    on_pick: impl Fn(usize, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let toggle_shared = shared.clone();
    let id = menu_id.to_owned();
    div()
        .relative()
        .child(controls::select_button(
            menu_id,
            t,
            value,
            open,
            width,
            move |_, _, _| {
                let mut shell = toggle_shared.borrow_mut();
                shell.page.open_menu = if shell.page.open_menu.as_deref() == Some(id.as_str()) {
                    None
                } else {
                    Some(id.clone())
                };
            },
        ))
        .when(open, move |div| {
            div.child(controls::select_menu(
                format!("{menu_id}-menu"),
                t,
                &options,
                selected,
                on_pick,
            ))
        })
}

#[allow(clippy::too_many_arguments)]
fn number_editor(
    state: &ShellState,
    shared: &Shared,
    window: &Window,
    t: Tokens,
    id: &'static str,
    current: String,
    value: i64,
    range: std::ops::RangeInclusive<i64>,
    set_id: String,
    control: Control,
) -> impl IntoElement {
    let focused = state.page.focus_step_value.is_focused(window);
    let shown = if focused && !state.page.step_value_fresh {
        state.page.step_value.as_str()
    } else {
        current.as_str()
    };
    let field = controls::Field {
        id,
        text: shown.to_owned(),
        placeholder: current.clone(),
        focus: state.page.focus_step_value.clone(),
        focused,
        width: Some(72.0),
        tokens: t,
    };
    let edit_shared = shared.clone();
    let edit_runtime = state.runtime.clone();
    let edit_set_id = set_id.clone();
    let delta_shared = shared.clone();
    let delta_runtime = state.runtime.clone();
    let delta_set_id = set_id;
    let lo = *range.start();
    let hi = *range.end();

    controls::number_field(
        field,
        move |edit, _, _| {
            let is_focused_edit = matches!(edit, Edit::Focused);
            let mut borrow = edit_shared.borrow_mut();
            let shell = &mut *borrow;
            controls::apply_edit(
                &mut shell.page.step_value,
                &mut shell.page.step_value_fresh,
                edit,
            );
            let buffer = shell.page.step_value.clone();
            drop(borrow);
            if !is_focused_edit && let Ok(parsed) = buffer.parse::<i64>() {
                let clamped = parsed.clamp(lo, hi);
                with_set_action(&edit_runtime, &edit_set_id, control, move |action, _| {
                    set_numeric(action, clamped);
                });
            }
        },
        move |delta, _, _| {
            let next = (value + delta).clamp(lo, hi);
            with_set_action(&delta_runtime, &delta_set_id, control, move |action, _| {
                set_numeric(action, next);
            });
            let mut shell = delta_shared.borrow_mut();
            shell.page.step_value = next.to_string();
            shell.page.step_value_fresh = true;
        },
    )
}

fn set_numeric(action: &mut Action, value: i64) {
    match action {
        Action::Volume { steps, .. } | Action::Brightness { steps, .. } => {
            *steps = value.clamp(1, u8::MAX as i64) as u8;
        }
        Action::Scroll { amount, .. } => {
            *amount = value.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn chord_picker(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    ids: &MenuIds,
    control: Control,
    set_id: String,
    chord: KeyChord,
) -> impl IntoElement {
    let ids = *ids;
    let key_options = KeyCode::ALL
        .iter()
        .map(|key| key.as_str().to_owned())
        .collect::<Vec<_>>();
    let key_selected = KeyCode::ALL
        .iter()
        .position(|key| *key == chord.key)
        .unwrap_or(0);
    let open = state.page.open_menu.as_deref() == Some(ids.chord_key);

    let key_shared = shared.clone();
    let key_runtime = state.runtime.clone();
    let key_id = set_id.clone();
    let pick_shared = shared.clone();

    let key_row = div()
        .relative()
        .child(controls::select_button(
            ids.chord_key,
            t,
            chord.key.as_str(),
            open,
            92.0,
            move |_, _, _| {
                let mut shell = key_shared.borrow_mut();
                shell.page.open_menu = if shell.page.open_menu.as_deref() == Some(ids.chord_key) {
                    None
                } else {
                    Some(ids.chord_key.to_owned())
                };
            },
        ))
        .when(open, |div| {
            div.child(controls::select_menu(
                format!("{}-menu", ids.chord_key),
                t,
                &key_options,
                key_selected,
                move |index, _, _| {
                    let picked = KeyCode::ALL[index];
                    with_set_action(&key_runtime, &key_id, control, move |action, _| {
                        if let Action::Shortcut { chord } = action {
                            chord.key = picked;
                        }
                    });
                    pick_shared.borrow_mut().page.open_menu = None;
                },
            ))
        });

    let mut modifiers = div().flex().items_center().gap_2();
    for (key, enabled) in [
        (TextKey::ModifierControl, chord.modifiers.control),
        (TextKey::ModifierAlt, chord.modifiers.alt),
        (TextKey::ModifierShift, chord.modifiers.shift),
        (TextKey::ModifierMeta, chord.modifiers.meta),
    ] {
        let modifier_shared = shared.clone();
        let modifier_runtime = state.runtime.clone();
        let modifier_id = set_id.clone();
        let wanted = !enabled;
        modifiers = modifiers.child(controls::toggle_labeled(
            format!("{}-mod-{key:?}", ids.chord_key),
            t,
            tr(language, key),
            enabled,
            move |_, _, _| {
                let _ = &modifier_shared;
                with_set_action(
                    &modifier_runtime,
                    &modifier_id,
                    control,
                    move |action, _| {
                        if let Action::Shortcut { chord } = action {
                            match key {
                                TextKey::ModifierControl => chord.modifiers.control = wanted,
                                TextKey::ModifierAlt => chord.modifiers.alt = wanted,
                                TextKey::ModifierShift => chord.modifiers.shift = wanted,
                                TextKey::ModifierMeta => chord.modifiers.meta = wanted,
                                _ => {}
                            }
                        }
                    },
                );
            },
        ));
    }

    div()
        .flex()
        .items_center()
        .gap_3()
        .child(key_row)
        .child(modifiers)
}

fn icon_text_button(
    t: Tokens,
    id: &'static str,
    icon: gpui_icons::LucideIcon,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
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
    id: &'static str,
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

// ---------------------------------------------------------------------------
// Pure helpers (mirrored from the eframe page; its tests live there until F4).
// ---------------------------------------------------------------------------

/// Static menu ids for one control row.
#[derive(Clone, Copy)]
struct MenuIds {
    kind: &'static str,
    dir: &'static str,
    axis: &'static str,
    macro_pick: &'static str,
    chord_key: &'static str,
    mouse_btn: &'static str,
    media: &'static str,
    steps: &'static str,
    amount: &'static str,
    path: &'static str,
}

impl MenuIds {
    fn of(control: Control) -> Self {
        match control {
            Control::Left => Self {
                kind: "kind-left",
                dir: "dir-left",
                axis: "axis-left",
                macro_pick: "macro-left",
                chord_key: "chord-left",
                mouse_btn: "mouse-left",
                media: "media-left",
                steps: "steps-left",
                amount: "amount-left",
                path: "path-left",
            },
            Control::Press => Self {
                kind: "kind-press",
                dir: "dir-press",
                axis: "axis-press",
                macro_pick: "macro-press",
                chord_key: "chord-press",
                mouse_btn: "mouse-press",
                media: "media-press",
                steps: "steps-press",
                amount: "amount-press",
                path: "path-press",
            },
            Control::Right => Self {
                kind: "kind-right",
                dir: "dir-right",
                axis: "axis-right",
                macro_pick: "macro-right",
                chord_key: "chord-right",
                mouse_btn: "mouse-right",
                media: "media-right",
                steps: "steps-right",
                amount: "amount-right",
                path: "path-right",
            },
        }
    }
}

/// Writes the sets back into the configuration (old page's `changed` flow).
fn commit_sets(runtime: &RuntimeHandle, sets: Vec<ActionSet>, active: Option<String>) {
    if let Err(error) = runtime.update_config(move |config| {
        config.action_sets = sets;
        let keep = active
            .filter(|id| config.action_sets.iter().any(|set| &set.id == id))
            .or_else(|| config.action_sets.first().map(|set| set.id.clone()));
        config.active_set = keep;
    }) {
        runtime.report_error(error.to_string());
    }
}

/// Clones the sets, applies `mutate` to the set with `set_id`, commits.
fn with_set(runtime: &RuntimeHandle, set_id: &str, mutate: impl FnOnce(&mut ActionSet)) {
    let mut sets = runtime.config().action_sets.clone();
    let active = runtime.config().active_set.clone();
    if let Some(set) = sets.iter_mut().find(|set| set.id == set_id) {
        mutate(set);
    }
    commit_sets(runtime, sets, active);
}

/// Clones the sets, applies `mutate` to one control's action, commits.
fn with_set_action(
    runtime: &RuntimeHandle,
    set_id: &str,
    control: Control,
    mutate: impl FnOnce(&mut Action, &Config),
) {
    let config = runtime.config();
    let mut sets = config.action_sets.clone();
    let active = config.active_set.clone();
    if let Some(set) = sets.iter_mut().find(|set| set.id == set_id) {
        let action = set.actions.for_control_mut(control);
        mutate(action, &config);
    }
    commit_sets(runtime, sets, active);
}

/// Swaps the selected set with its neighbour; returns the new index.
fn move_sets(runtime: &RuntimeHandle, shared: &Shared, delta: i64) -> Option<usize> {
    let selected = shared.borrow().page.selected_set.clone()?;
    let mut sets = runtime.config().action_sets.clone();
    let active = runtime.config().active_set.clone();
    let index = sets.iter().position(|set| set.id == selected)?;
    let target = index as i64 + delta;
    if target < 0 || target as usize >= sets.len() {
        return None;
    }
    sets.swap(index, target as usize);
    commit_sets(runtime, sets, active);
    Some(target as usize)
}

fn new_set(sets: &[ActionSet], language: Language) -> ActionSet {
    let base = tr(language, TextKey::SetDefaultName);
    let mut number = 1;
    loop {
        let name = format!("{base} {number}");
        let id = format!("set-{number}");
        if !sets.iter().any(|set| set.name == name || set.id == id) {
            return ActionSet::new(id, name, SetIcon::Star);
        }
        number += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    None,
    Volume,
    Mute,
    Brightness,
    Scroll,
    Macro,
    Shortcut,
    Mouse,
    Media,
    Launch,
}

impl Kind {
    const ALL: [Self; 10] = [
        Self::None,
        Self::Volume,
        Self::Mute,
        Self::Brightness,
        Self::Scroll,
        Self::Macro,
        Self::Shortcut,
        Self::Mouse,
        Self::Media,
        Self::Launch,
    ];

    fn choices(brightness_supported: bool) -> Vec<Self> {
        Self::ALL
            .iter()
            .copied()
            .filter(|kind| brightness_supported || *kind != Self::Brightness)
            .collect()
    }

    fn of(action: &Action) -> Self {
        match action {
            Action::None => Self::None,
            Action::Volume { .. } => Self::Volume,
            Action::Mute => Self::Mute,
            Action::Brightness { .. } => Self::Brightness,
            Action::Scroll { .. } => Self::Scroll,
            Action::Macro { .. } => Self::Macro,
            Action::Shortcut { .. } => Self::Shortcut,
            Action::MouseClick { .. } => Self::Mouse,
            Action::Media { .. } => Self::Media,
            Action::Launch { .. } => Self::Launch,
        }
    }

    fn label(self, language: Language) -> &'static str {
        match self {
            Self::None => tr(language, TextKey::ActionNone),
            Self::Volume => tr(language, TextKey::ActionKindVolume),
            Self::Mute => tr(language, TextKey::ActionKindMute),
            Self::Brightness => tr(language, TextKey::ActionKindBrightness),
            Self::Scroll => tr(language, TextKey::ActionKindScroll),
            Self::Macro => tr(language, TextKey::ActionKindMacro),
            Self::Shortcut => tr(language, TextKey::ActionKindShortcut),
            Self::Mouse => tr(language, TextKey::ActionKindMouse),
            Self::Media => tr(language, TextKey::ActionKindMedia),
            Self::Launch => tr(language, TextKey::ActionKindLaunch),
        }
    }

    fn default_action(self, config: &Config) -> Action {
        match self {
            Self::None => Action::None,
            Self::Mute => Action::Mute,
            Self::Volume => Action::Volume {
                direction: VolumeDirection::Up,
                steps: 1,
            },
            Self::Brightness => Action::Brightness {
                direction: VolumeDirection::Up,
                steps: 1,
            },
            Self::Scroll => Action::Scroll {
                axis: ScrollAxis::Vertical,
                amount: -3,
            },
            Self::Macro => match config.macros.first() {
                Some(definition) => Action::Macro {
                    name: definition.name.clone(),
                },
                None => Action::None,
            },
            Self::Shortcut => Action::Shortcut {
                chord: KeyChord::new(KeyCode::F13),
            },
            Self::Mouse => Action::MouseClick {
                button: MouseButton::Left,
            },
            Self::Media => Action::Media {
                command: MediaCommand::PlayPause,
            },
            Self::Launch => Action::Launch {
                path: PathBuf::from("."),
            },
        }
    }
}

fn control_label(language: Language, control: Control) -> &'static str {
    match control {
        Control::Left => tr(language, TextKey::ControlLeft),
        Control::Right => tr(language, TextKey::ControlRight),
        Control::Press => tr(language, TextKey::ControlPress),
    }
}

fn direction_label(language: Language, direction: VolumeDirection) -> &'static str {
    match direction {
        VolumeDirection::Up => tr(language, TextKey::DirectionUp),
        VolumeDirection::Down => tr(language, TextKey::DirectionDown),
    }
}

fn axis_label(language: Language, axis: ScrollAxis) -> &'static str {
    match axis {
        ScrollAxis::Vertical => tr(language, TextKey::AxisVertical),
        ScrollAxis::Horizontal => tr(language, TextKey::AxisHorizontal),
    }
}

fn media_label(language: Language, command: MediaCommand) -> &'static str {
    match command {
        MediaCommand::PlayPause => tr(language, TextKey::ActionMediaPlayPause),
        MediaCommand::Next => tr(language, TextKey::ActionMediaNext),
        MediaCommand::Previous => tr(language, TextKey::ActionMediaPrevious),
        MediaCommand::Stop => tr(language, TextKey::ActionMediaStop),
    }
}

/// Clickable wheel preview above the set list (ported from the eframe page;
/// geometry uses the full box like the old preview did).
fn preview_box(
    state: &ShellState,
    shared: &Shared,
    cx: &mut Context<SettingsView>,
    sets: &[ActionSet],
    selected: Option<usize>,
) -> impl IntoElement {
    const BOX: f32 = 300.0;
    let geometry = WheelGeometry::from_size(BOX);
    let center = point(px(BOX / 2.0), px(BOX / 2.0));
    let origin = wheel_paint::origin(center, &geometry);
    let selected_index = selected.unwrap_or(0);
    let hovered = state.page.preview_hovered;
    let title_y = BOX - 8.0;
    let paint_sets = sets.to_vec();

    let move_shared = shared.clone();
    let move_weak = cx.weak_entity();
    let down_shared = shared.clone();
    let down_weak = cx.weak_entity();

    div()
        .id("set-preview")
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
                        selected_index,
                        hovered,
                    );
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(wheel_paint::children(
            &geometry,
            origin,
            title_y,
            BOX,
            sets,
            selected_index,
        ))
        .on_mouse_move(move |event, _, cx| {
            let count = move_shared.borrow().runtime.config().action_sets.len();
            let hovered = wheel_paint::sector_at(&geometry, count, origin, event.position);
            let mut state = move_shared.borrow_mut();
            if state.page.preview_hovered != hovered {
                state.page.preview_hovered = hovered;
                if let Some(view) = move_weak.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            }
        })
        .on_mouse_down(gpui::MouseButton::Left, move |event, _, cx| {
            let count = down_shared.borrow().runtime.config().action_sets.len();
            let hit = wheel_paint::sector_at(&geometry, count, origin, event.position);
            let Some(index) = hit else {
                return;
            };
            let id = down_shared
                .borrow()
                .runtime
                .config()
                .action_sets
                .get(index)
                .map(|set| set.id.clone());
            if let Some(id) = id {
                down_shared.borrow_mut().page.selected_set = Some(id);
                if let Some(view) = down_weak.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_have_unique_names_and_ids() {
        let mut sets = Vec::new();
        for _ in 0..3 {
            let set = new_set(&sets, Language::En);
            sets.push(set);
        }
        assert_eq!(sets[0].id, "set-1");
        assert_eq!(sets[1].id, "set-2");
        assert_eq!(sets[2].name, "Set 3");
    }

    #[test]
    fn brightness_is_only_offered_when_supported() {
        let without = Kind::choices(false);
        assert!(without.contains(&Kind::None));
        assert!(!without.contains(&Kind::Brightness));

        let with = Kind::choices(true);
        assert!(with.contains(&Kind::Brightness));
        assert_eq!(with.first(), Some(&Kind::None));
    }

    #[test]
    fn kind_round_trips_every_action() {
        let mut config = Config::default();
        config.macros.push(crate::core::MacroDefinition {
            name: "macro".to_owned(),
            steps: vec![crate::core::MacroStep::Delay { ms: 0 }],
        });

        for kind in Kind::ALL {
            let action = kind.default_action(&config);
            assert_eq!(Kind::of(&action), kind, "kind mismatch for {kind:?}");
        }
    }
}
