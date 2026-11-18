//! Actions page: manage the action sets shown in the wheel (gpui port).

use crate::app::RuntimeHandle;
use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::{
    Action, ActionSet, Config, Control, KeyChord, KeyCode, Language, SetIcon, TextKey,
    WheelGeometry, tr,
};
use crate::ui::capture::{self, CaptureTarget};
use crate::ui::controls;
use crate::ui::iconset;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use crate::ui::wheel_paint;
use gpui::prelude::*;
use gpui::{AnyElement, App, AsyncApp, Context, PathPromptOptions, Window, canvas, div, point, px};
use gpui_kit::component::{WindowExt, button::ButtonVariant};
use std::path::PathBuf;

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
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

    let wide = window.viewport_size().width >= px(1100.0);

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabActions),
            tr(language, TextKey::ActionsSubtitle),
        ))
        .child(
            div()
                .flex()
                .when(wide, |node| node.flex_row().items_start().gap(px(16.0)))
                .when(!wide, |node| node.flex_col().gap(px(10.0)))
                .child(
                    div()
                        .when(wide, |node| node.flex_1().min_w(px(380.0)).max_w(px(640.0)))
                        .when(!wide, |node| node.w_full())
                        .child(set_column(state, shared, t, language, &sets, selected)),
                )
                .child(
                    div()
                        .when(wide, |node| node.flex_1().min_w(px(420.0)))
                        .when(!wide, |node| node.w_full())
                        .child(editor_column(
                            state, shared, window, t, language, &config, &sets, &active, selected,
                            &mut *cx,
                        )),
                ),
        )
}

// ---------------------------------------------------------------------------
// Left column: set list and ordering buttons.
// ---------------------------------------------------------------------------

fn set_column(
    state: &ShellState,
    shared: &Shared,
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
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
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

    let add_shared = shared.clone();
    let add_runtime = state.runtime.clone();
    let left_shared = shared.clone();
    let left_runtime = state.runtime.clone();
    let right_shared = shared.clone();
    let right_runtime = state.runtime.clone();

    let preview = preview_box(sets, selected);

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().w_full().flex().justify_center().child(preview))
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
                .child(controls::icon_text_button(
                    "add-set",
                    t,
                    gpui_icons::LucideIcon::Plus,
                    tr(language, TextKey::AddSetButton),
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
                .when(selected.is_some(), |div| {
                    div.child(controls::icon_button(
                        "move-set-left",
                        t,
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
                .when(selected.is_some(), |div| {
                    div.child(controls::icon_button(
                        "move-set-right",
                        t,
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
    window: &mut Window,
    t: Tokens,
    language: Language,
    config: &Config,
    sets: &[ActionSet],
    active: &Option<String>,
    selected: Option<usize>,
    cx: &mut App,
) -> AnyElement {
    if sets.is_empty() {
        return controls::card(
            "set-editor-empty",
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

    let delete = controls::danger_button(
        "delete-set",
        gpui_icons::LucideIcon::Trash,
        tr(language, TextKey::DeleteSetButton),
        move |_, window, cx| {
            // The dialog builder can run more than once, so it gets owned
            // copies instead of borrowing values from this click closure.
            let dialog_shared = delete_shared.clone();
            let dialog_runtime = delete_runtime.clone();
            let dialog_id = delete_id.clone();
            window.open_alert_dialog(cx, move |alert, _, _| {
                let ok_shared = dialog_shared.clone();
                let ok_runtime = dialog_runtime.clone();
                let ok_id = dialog_id.clone();
                alert
                    .confirm()
                    .title(tr(language, TextKey::ConfirmDeleteSetTitle))
                    .description(tr(language, TextKey::ConfirmDeleteSetBody))
                    .ok_text(tr(language, TextKey::ConfirmDelete))
                    .cancel_text(tr(language, TextKey::CancelAction))
                    .ok_variant(ButtonVariant::Danger)
                    .on_ok(move |_, _, _| {
                        let mut sets = ok_runtime.config().action_sets.clone();
                        let mut active = ok_runtime.config().active_set.clone();
                        let Some(position) = sets.iter().position(|set| set.id == ok_id) else {
                            return true;
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
                        commit_sets(&ok_runtime, sets, active);
                        ok_shared.borrow_mut().page.selected_set = next.and_then(|index| {
                            ok_runtime
                                .config()
                                .action_sets
                                .get(index)
                                .map(|set| set.id.clone())
                        });
                        true
                    })
            });
        },
    )
    .into_any_element();

    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(controls::card(
            "set-editor",
            t,
            editor(
                state, shared, window, t, language, config, set, is_active, brightness, delete, cx,
            ),
        ))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn editor(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    t: Tokens,
    language: Language,
    config: &Config,
    set: &ActionSet,
    is_active: bool,
    brightness_supported: bool,
    delete: AnyElement,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let set_id = set.id.clone();

    // Name row.
    let name_field = controls::field_input(
        "set-name",
        tr(language, TextKey::SetNameLabel),
        set.name.as_str(),
        Some(220.0),
        window,
        cx,
    );

    // Icon picker.
    let icon_cells = SetIcon::ALL
        .iter()
        .map(|icon| {
            let is_selected = set.icon == *icon;
            let icon_runtime = state.runtime.clone();
            let icon_id = set_id.clone();
            let icon = *icon;
            div()
                .id(format!("set-icon-{}", iconset::name(icon)))
                .flex()
                .items_center()
                .justify_center()
                .w_full()
                .h(px(40.0))
                .rounded(px(7.0))
                .border_1()
                .border_color(if is_selected { style::ACCENT } else { t.card })
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
                    icon,
                    22.0,
                    if is_selected { style::ACCENT } else { t.text },
                ))
                .on_click(move |_, _, _| {
                    with_set(&icon_runtime, &icon_id, move |set| set.icon = icon);
                })
        })
        .collect::<Vec<_>>();
    let icon_grid = div()
        .w_full()
        .grid()
        .grid_cols(10)
        .gap_1()
        .children(icon_cells);

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
                cx,
            )
            .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_col()
        .gap_2()
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
                })
                .child(div().flex_1().min_w(px(0.0)))
                .child(delete),
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
                .child(icon_grid),
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
    window: &mut Window,
    t: Tokens,
    language: Language,
    config: &Config,
    set: &ActionSet,
    control: Control,
    brightness_supported: bool,
    cx: &mut App,
) -> impl IntoElement + use<> {
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
                            200.0,
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
                                200.0,
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
                .mt(px(2.0))
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .children(parameters(
                    state, shared, window, t, language, config, set, control, ids, cx,
                )),
        )
}

#[allow(clippy::too_many_arguments)]
fn parameters(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    t: Tokens,
    language: Language,
    config: &Config,
    set: &ActionSet,
    control: Control,
    ids: MenuIds,
    cx: &mut App,
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
                    window,
                    ids.steps,
                    i64::from(steps),
                    1..=10,
                    set.id.clone(),
                    control,
                    cx,
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
                    window,
                    ids.amount,
                    i64::from(amount),
                    -100..=100,
                    set.id.clone(),
                    control,
                    cx,
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
            out.push(
                controls::field_input(
                    ids.path,
                    "…",
                    path.to_string_lossy().as_ref(),
                    Some(320.0),
                    window,
                    cx,
                )
                .into_any_element(),
            );

            let browse_runtime = state.runtime.clone();
            let browse_id = set.id.clone();
            out.push(
                controls::secondary_button(
                    t,
                    tr(language, TextKey::BrowseButton),
                    move |_, _, cx| {
                        let receiver = cx.prompt_for_paths(PathPromptOptions {
                            files: true,
                            directories: false,
                            multiple: false,
                            prompt: None,
                        });
                        let runtime = browse_runtime.clone();
                        let set_id = browse_id.clone();
                        cx.spawn(async move |cx: &mut AsyncApp| {
                            if let Ok(Ok(Some(paths))) = receiver.await
                                && let Some(path) = paths.into_iter().next()
                            {
                                cx.update(|_| {
                                    with_set_action(
                                        &runtime,
                                        &set_id,
                                        control,
                                        move |action, _| {
                                            if let Action::Launch { path: slot } = action {
                                                *slot = path;
                                            }
                                        },
                                    );
                                });
                            }
                        })
                        .detach();
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
                width,
                on_pick,
            ))
        })
}

#[allow(clippy::too_many_arguments)]
fn number_editor(
    state: &ShellState,
    window: &mut Window,
    id: &'static str,
    value: i64,
    range: std::ops::RangeInclusive<i64>,
    set_id: String,
    control: Control,
    cx: &mut App,
) -> AnyElement {
    let delta_runtime = state.runtime.clone();
    let lo = *range.start();
    let hi = *range.end();

    controls::number_field(
        id,
        value.to_string(),
        move |delta, _, _| {
            let next = (value + delta).clamp(lo, hi);
            with_set_action(&delta_runtime, &set_id, control, move |action, _| {
                set_numeric(action, next);
            });
        },
        window,
        cx,
    )
}

pub(crate) fn set_numeric(action: &mut Action, value: i64) {
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
    let target = CaptureTarget::Shortcut {
        set_id: set_id.clone(),
        control,
    };
    let (capturing, label) =
        capture::chip(state, &target, chord, tr(language, TextKey::PressKeyHint));

    let key_shared = shared.clone();
    let confirm_shared = shared.clone();
    let cancel_shared = shared.clone();

    div()
        .flex()
        .items_center()
        .gap_3()
        .child(controls::key_capture_button(
            ids.chord_key.to_owned(),
            t,
            &label,
            capturing,
            move |_, window, cx| {
                capture::start(window, cx, &key_shared, target.clone());
            },
        ))
        .when(capturing && state.capture_pending.is_some(), |row| {
            row.child(controls::capture_buttons(
                ids.chord_key,
                t,
                move |_, _, _| capture::confirm(&confirm_shared),
                move |_, _, _| capture::cancel(&cancel_shared),
            ))
        })
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
pub(crate) fn with_set(runtime: &RuntimeHandle, set_id: &str, mutate: impl FnOnce(&mut ActionSet)) {
    let mut sets = runtime.config().action_sets.clone();
    let active = runtime.config().active_set.clone();
    if let Some(set) = sets.iter_mut().find(|set| set.id == set_id) {
        mutate(set);
    }
    commit_sets(runtime, sets, active);
}

/// Clones the sets, applies `mutate` to one control's action, commits.
pub(crate) fn with_set_action(
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

/// Stores a key captured in the settings window as the shortcut of a control.
pub(crate) fn capture_chord(
    runtime: &RuntimeHandle,
    set_id: &str,
    control: Control,
    captured: KeyChord,
) {
    with_set_action(runtime, set_id, control, move |action, _| {
        if let Action::Shortcut { chord } = action {
            *chord = captured;
        }
    });
}

/// Moves the selected set by one place, wrapping around both ends; returns
/// its new index.
fn move_sets(runtime: &RuntimeHandle, shared: &Shared, delta: i64) -> Option<usize> {
    let selected = shared.borrow().page.selected_set.clone()?;
    let mut sets = runtime.config().action_sets.clone();
    let active = runtime.config().active_set.clone();
    let index = sets.iter().position(|set| set.id == selected)?;
    let len = sets.len() as i64;
    let target = (index as i64 + delta).rem_euclid(len) as usize;
    let set = sets.remove(index);
    sets.insert(target, set);
    commit_sets(runtime, sets, active);
    Some(target)
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
fn preview_box(sets: &[ActionSet], selected: Option<usize>) -> impl IntoElement {
    const BOX: f32 = 300.0;
    let geometry = WheelGeometry::from_size(BOX);
    let center = point(px(BOX / 2.0), px(BOX / 2.0));
    let origin = wheel_paint::origin(center, &geometry);
    let selected_index = selected.unwrap_or(0);
    let paint_sets = sets.to_vec();
    let title_y = BOX - 8.0;

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
                    // The preview mirrors the wheel without the mouse: no
                    // hover highlight, no click selection.
                    wheel_paint::paint(
                        window,
                        &geometry,
                        paint_center,
                        &paint_sets,
                        selected_index,
                        None,
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
            false,
        ))
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
