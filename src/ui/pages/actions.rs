//! Actions page: manage the action sets shown in the wheel (gpui port).
//!
//! Layout: a fixed left column (static wheel preview, set list, new-set
//! button) and a scrolling right column editing the selected set. The wheel
//! is a preview only; selection happens through the list.

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

/// Viewport width from which the page shows its two-column layout.
const WIDE: f32 = 1240.0;

/// Width of the fixed left column (wheel and set list).
const LEFT_WIDTH: f32 = 340.0;

/// Width of the dropdown menus opened from a full-width select.
const MENU_WIDTH: f32 = 300.0;

/// Width of the label column inside a parameter row.
const PARAM_LABEL_WIDTH: f32 = 90.0;

/// Transparent border for list rows, so selected and unselected rows share
/// one height.
const ROW_BORDER_HIDDEN: gpui::Rgba = style::TRANSPARENT;

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> AnyElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;
    let config = state.runtime.config();
    let sets = config.action_sets.clone();

    let selected = state
        .page
        .selected_set
        .as_deref()
        .and_then(|id| sets.iter().position(|set| set.id == id))
        .or(if sets.is_empty() { None } else { Some(0) });

    let wide = window.viewport_size().width >= px(WIDE);

    div()
        .id("actions-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .gap(px(12.0))
        .when(!wide, |node| node.overflow_scroll())
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabActions),
            tr(language, TextKey::ActionsSubtitle),
        ))
        .child(
            div()
                .flex()
                .flex_1()
                .min_h(px(0.0))
                .when(wide, |node| node.flex_row().gap(px(24.0)))
                .when(!wide, |node| node.flex_col().gap(px(12.0)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .when(wide, |node| node.w(px(LEFT_WIDTH)).flex_none())
                        .when(!wide, |node| node.w_full())
                        .child(left_column(
                            state, shared, t, language, &sets, selected, wide,
                        )),
                )
                .child(
                    div()
                        .id("actions-editor")
                        .flex()
                        .flex_col()
                        .when(wide, |node| {
                            node.flex_1().min_w(px(0.0)).overflow_y_scroll()
                        })
                        .when(!wide, |node| node.w_full())
                        .child(right_column(
                            state, shared, window, t, language, &config, &sets, selected, &mut *cx,
                        )),
                ),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Left column: static wheel preview, set list and the new-set button.
// ---------------------------------------------------------------------------

fn left_column(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    sets: &[ActionSet],
    selected: Option<usize>,
    wide: bool,
) -> AnyElement {
    let rows = sets
        .iter()
        .enumerate()
        .map(|(index, set)| {
            set_row(state, shared, t, language, set, selected == Some(index)).into_any_element()
        })
        .collect::<Vec<_>>();

    let list = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .w_full()
                .rounded(px(style::RADIUS_CARD))
                .border_1()
                .border_color(t.border)
                .bg(t.card)
                .p(px(6.0))
                .flex()
                .flex_col()
                .gap_1()
                .children(rows),
        )
        .when(sets.is_empty(), |section| {
            section.child(
                div()
                    .text_size(px(12.5))
                    .text_color(t.muted)
                    .child(tr(language, TextKey::EmptyActionsBody)),
            )
        })
        .child(new_set_button(state, shared, t, language));

    // On wide screens the wheel never scrolls: only the list area does.
    div()
        .flex()
        .flex_col()
        .gap_2()
        .when(wide, |node| node.flex_1().min_h(px(0.0)))
        .child(
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(preview_box(sets, selected)),
        )
        .child(
            div()
                .id("set-list-scroll")
                .flex()
                .flex_col()
                .gap_2()
                .when(wide, |node| {
                    node.flex_1().min_h(px(0.0)).overflow_y_scroll()
                })
                .when(!wide, |node| node.w_full())
                .child(list),
        )
        .into_any_element()
}

/// One set in the list: icon, name and, while hovered, the reorder buttons.
/// The buttons live in a slot that always has the same width, so rows never
/// change size when the pointer moves.
fn set_row(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    set: &ActionSet,
    is_selected: bool,
) -> impl IntoElement {
    let id = set.id.clone();
    let row_shared = shared.clone();
    let hover_shared = shared.clone();
    let hover_id = set.id.clone();
    let up_shared = shared.clone();
    let up_runtime = state.runtime.clone();
    let up_id = set.id.clone();
    let down_shared = shared.clone();
    let down_runtime = state.runtime.clone();
    let down_id = set.id.clone();
    let sets_len = state.runtime.config().action_sets.len();
    let is_hovered = state.page.hovered_set.as_deref() == Some(set.id.as_str());

    div()
        .id(format!("set-row-{}", set.id))
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py(px(7.0))
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(if is_selected {
            style::ACCENT
        } else {
            ROW_BORDER_HIDDEN
        })
        .bg(if is_selected { t.accent_soft } else { t.card })
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
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(4.0))
                .w(px(68.0))
                .flex_none()
                // The buttons are always mounted so rows never change size;
                // without hover they are simply transparent.
                .when(sets_len > 1, |div| {
                    let tone = if is_hovered {
                        t.text
                    } else {
                        style::TRANSPARENT
                    };
                    div.child(controls::ghost_icon_button(
                        format!("move-up-{}", set.id),
                        gpui_icons::LucideIcon::ChevronUp,
                        tr(language, TextKey::MoveSetUp),
                        tone,
                        move |_, window, cx| {
                            window.blur(cx);
                            move_set(&up_runtime, &up_shared, &up_id, -1);
                        },
                    ))
                    .child(controls::ghost_icon_button(
                        format!("move-down-{}", set.id),
                        gpui_icons::LucideIcon::ChevronDown,
                        tr(language, TextKey::MoveSetDown),
                        tone,
                        move |_, window, cx| {
                            window.blur(cx);
                            move_set(&down_runtime, &down_shared, &down_id, 1);
                        },
                    ))
                }),
        )
        .on_hover(move |hovered, _, _| {
            let mut shell = hover_shared.borrow_mut();
            if *hovered {
                shell.page.hovered_set = Some(hover_id.clone());
            } else if shell.page.hovered_set.as_deref() == Some(hover_id.as_str()) {
                shell.page.hovered_set = None;
            }
        })
        .on_click(move |_, window, cx| {
            window.blur(cx);
            let mut shell = row_shared.borrow_mut();
            shell.page.selected_set = Some(id.clone());
            shell.page.open_menu = None;
        })
}

/// Full-width dashed placeholder that creates the next group.
fn new_set_button(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
) -> impl IntoElement {
    let add_shared = shared.clone();
    let add_runtime = state.runtime.clone();
    controls::dashed_button(
        "add-set",
        t,
        tr(language, TextKey::AddSetButton),
        move |_, window, cx| {
            window.blur(cx);
            let mut sets = add_runtime.config().action_sets.clone();
            let set_language = add_runtime.config().language;
            let set = new_set(&sets, set_language);
            let id = set.id.clone();
            sets.push(set);
            commit_sets(&add_runtime, sets, add_runtime.config().active_set.clone());
            let mut shell = add_shared.borrow_mut();
            shell.page.selected_set = Some(id);
            shell.page.open_menu = None;
        },
    )
}

// ---------------------------------------------------------------------------
// Right column: editor for the selected set.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn right_column(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    t: Tokens,
    language: Language,
    config: &Config,
    sets: &[ActionSet],
    selected: Option<usize>,
    cx: &mut App,
) -> AnyElement {
    if sets.is_empty() {
        return controls::card(
            "set-editor-empty",
            t,
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(15.0))
                        .text_color(t.text)
                        .child(tr(language, TextKey::EmptyActionsTitle)),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(t.muted)
                        .child(tr(language, TextKey::EmptyActionsBody)),
                ),
        )
        .into_any_element();
    }

    let index = selected.unwrap_or(0);
    let Some(set) = sets.get(index) else {
        return div().into_any_element();
    };
    let brightness = state.platform.brightness_supported;

    controls::card(
        "set-editor",
        t,
        editor(
            state, shared, window, t, language, config, set, brightness, cx,
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
    config: &Config,
    set: &ActionSet,
    brightness_supported: bool,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let set_id = set.id.clone();

    // Icon chip: opens the icon picker dialog.
    let chip_runtime = state.runtime.clone();
    let chip_id = set_id.clone();
    let chip_language = language;
    let chip_tokens = t;
    let icon_chip = div()
        .id("set-icon-chip")
        .relative()
        .size(px(52.0))
        .cursor_pointer()
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(10.0))
                .border_1()
                .border_color(t.border)
                .bg(t.card)
                .hover(|div| div.border_color(style::ACCENT))
                .child(iconset::element(set.icon, 22.0, t.text)),
        )
        .child(
            div()
                .absolute()
                .right(px(-4.0))
                .bottom(px(-4.0))
                .size(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(t.border)
                .bg(t.bg)
                .child(
                    gpui_icons::lucide(gpui_icons::LucideIcon::Pencil)
                        .size(px(10.0))
                        .text_color(t.muted),
                ),
        )
        .on_click(move |_, window, cx| {
            open_icon_dialog(
                window,
                cx,
                chip_tokens,
                &chip_runtime,
                &chip_id,
                chip_language,
            );
        });

    // Name field with its label above, like the mockups.
    let name_field = controls::field_input("set-name", "", set.name.as_str(), None, window, cx);

    // Delete stays a quiet icon button; the confirmation dialog is unchanged.
    let delete_shared = shared.clone();
    let delete_runtime = state.runtime.clone();
    let delete_id = set.id.clone();
    let delete = controls::icon_button_tinted(
        "delete-set",
        t,
        gpui_icons::LucideIcon::Trash,
        tr(language, TextKey::DeleteSetButton),
        style::DANGER,
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
    );

    let gesture_cards = [Control::Left, Control::Press, Control::Right]
        .iter()
        .map(|control| {
            gesture_card(
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
        .w_full()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .w_full()
                .flex()
                .items_end()
                .gap(px(12.0))
                .child(icon_chip)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(t.muted)
                                .child(tr(language, TextKey::SetNameLabel)),
                        )
                        .child(name_field),
                )
                .child(delete),
        )
        .child(super::separator(t))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(t.muted)
                .child(tr(language, TextKey::GesturesLabel)),
        )
        .children(gesture_cards)
}

// ---------------------------------------------------------------------------
// Gesture cards and parameters.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn gesture_card(
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
    let action = set.actions.for_control(control);
    let kind = Kind::of(action);
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
    let kind_icons = choices
        .iter()
        .map(|choice| choice.icon())
        .collect::<Vec<_>>();

    let pick_shared = shared.clone();
    let pick_runtime = state.runtime.clone();
    let pick_id = set.id.clone();

    let kind_select = enum_select(
        shared,
        t,
        ids.kind,
        Some(kind_icons),
        kind.label(language),
        open,
        options,
        selected,
        move |index, _, _| {
            let choice = choices[index];
            with_set_action(&pick_runtime, &pick_id, control, |action, cfg| {
                *action = choice.default_action(cfg)
            });
            pick_shared.borrow_mut().page.open_menu = None;
        },
    )
    .into_any_element();

    div()
        .w_full()
        .flex()
        .gap(px(16.0))
        .rounded(px(style::RADIUS_CARD))
        .border_1()
        .border_color(t.border)
        .bg(t.card)
        .p(px(12.0))
        .child(
            div()
                .w(px(140.0))
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .size(px(40.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(style::RADIUS_CONTROL))
                        .bg(t.accent_soft)
                        .child(
                            gpui_icons::lucide(control_icon(control))
                                .size(px(18.0))
                                .text_color(style::ACCENT),
                        ),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(13.5))
                        .text_color(t.text)
                        .child(control_label(language, control)),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(param_row(
                    t,
                    tr(language, TextKey::GestureActionLabel),
                    kind_select,
                ))
                .children(parameters(
                    state, shared, window, t, language, config, set, control, ids, cx,
                )),
        )
}

fn control_icon(control: Control) -> gpui_icons::LucideIcon {
    match control {
        Control::Left => gpui_icons::LucideIcon::RotateCcw,
        Control::Right => gpui_icons::LucideIcon::RotateCw,
        Control::Press => gpui_icons::LucideIcon::CircleDot,
    }
}

/// One parameter line: muted label column, then the control.
fn param_row(t: Tokens, label: &'static str, control: AnyElement) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .w(px(PARAM_LABEL_WIDTH))
                .flex_none()
                .text_size(px(13.0))
                .text_color(t.muted)
                .child(label),
        )
        .child(control)
        .into_any_element()
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
        // The kind select above already spells these out; no extra line.
        Action::None | Action::Mute => {}
        Action::Volume { direction, steps } | Action::Brightness { direction, steps } => {
            out.push(param_row(
                t,
                tr(language, TextKey::ActionDirection),
                enum_select(
                    shared,
                    t,
                    ids.dir,
                    Some(vec![
                        gpui_icons::LucideIcon::ArrowUp,
                        gpui_icons::LucideIcon::ArrowDown,
                    ]),
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
            ));
            out.push(param_row(
                t,
                tr(language, TextKey::ActionSteps),
                number_editor(
                    state,
                    window,
                    ids.steps,
                    i64::from(steps),
                    1..=10,
                    set.id.clone(),
                    control,
                    cx,
                ),
            ));
        }
        Action::Scroll { axis, amount } => {
            out.push(param_row(
                t,
                tr(language, TextKey::ActionAxis),
                enum_select(
                    shared,
                    t,
                    ids.axis,
                    Some(vec![
                        gpui_icons::LucideIcon::ArrowUpDown,
                        gpui_icons::LucideIcon::ArrowLeftRight,
                    ]),
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
            ));
            out.push(param_row(
                t,
                tr(language, TextKey::ActionAmount),
                number_editor(
                    state,
                    window,
                    ids.amount,
                    i64::from(amount),
                    -100..=100,
                    set.id.clone(),
                    control,
                    cx,
                ),
            ));
        }
        Action::Macro { name } => {
            let options = config
                .macros
                .iter()
                .map(|definition| definition.name.clone())
                .collect::<Vec<_>>();
            let selected = options.iter().position(|option| *option == name);
            out.push(param_row(
                t,
                tr(language, TextKey::ActionMacroName),
                enum_select(
                    shared,
                    t,
                    ids.macro_pick,
                    Some(vec![gpui_icons::LucideIcon::ListChecks; options.len()]),
                    if name.is_empty() { "—" } else { &name },
                    state.page.open_menu.as_deref() == Some(ids.macro_pick),
                    options.clone(),
                    selected.unwrap_or(0),
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
            ));
        }
        Action::Shortcut { chord } => {
            out.push(
                param_row(
                    t,
                    tr(language, TextKey::ActionChord),
                    chord_chip(
                        state,
                        shared,
                        t,
                        language,
                        &ids,
                        control,
                        set.id.clone(),
                        chord,
                    ),
                )
                .into_any_element(),
            );
        }
        Action::MouseClick { button } => {
            let buttons = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
            let selected = buttons
                .iter()
                .position(|option| *option == button)
                .unwrap_or(0);
            out.push(param_row(
                t,
                tr(language, TextKey::ActionButton),
                enum_select(
                    shared,
                    t,
                    ids.mouse_btn,
                    Some(vec![gpui_icons::LucideIcon::MousePointer; 3]),
                    button.label(language),
                    state.page.open_menu.as_deref() == Some(ids.mouse_btn),
                    buttons
                        .iter()
                        .map(|option| option.label(language).to_owned())
                        .collect(),
                    selected,
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
            ));
        }
        Action::Media { command } => {
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
            out.push(param_row(
                t,
                tr(language, TextKey::ActionCommand),
                enum_select(
                    shared,
                    t,
                    ids.media,
                    Some(vec![
                        gpui_icons::LucideIcon::Play,
                        gpui_icons::LucideIcon::SkipForward,
                        gpui_icons::LucideIcon::SkipBack,
                        gpui_icons::LucideIcon::Square,
                    ]),
                    media_label(language, command),
                    state.page.open_menu.as_deref() == Some(ids.media),
                    commands
                        .iter()
                        .map(|option| media_label(language, *option).to_owned())
                        .collect(),
                    selected,
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
            ));
        }
        Action::Launch { path } => {
            let browse_runtime = state.runtime.clone();
            let browse_id = set.id.clone();
            out.push(
                param_row(
                    t,
                    tr(language, TextKey::ActionPath),
                    div()
                        .flex()
                        .items_center()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap_2()
                        .child(controls::field_input(
                            ids.path,
                            "…",
                            path.to_string_lossy().as_ref(),
                            None,
                            window,
                            cx,
                        ))
                        .child(controls::secondary_button(
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
                        ))
                        .into_any_element(),
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

/// Dropdown for one parameter; the pick closure carries the concrete value.
/// `icons` is parallel to `options`: the button shows the selected entry's
/// icon and the menu shows one per entry. The button fills the row; the menu
/// uses [`MENU_WIDTH`].
#[allow(clippy::too_many_arguments)]
fn enum_select(
    shared: &Shared,
    t: Tokens,
    menu_id: &'static str,
    icons: Option<Vec<gpui_icons::LucideIcon>>,
    value: &str,
    open: bool,
    options: Vec<String>,
    selected: usize,
    on_pick: impl Fn(usize, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let toggle_shared = shared.clone();
    let id = menu_id.to_owned();
    let button_icon = icons
        .as_ref()
        .and_then(|icons| icons.get(selected).copied());
    div()
        .relative()
        .flex_1()
        .min_w(px(0.0))
        .child(controls::select_button_with_icon(
            menu_id,
            t,
            button_icon,
            value,
            open,
            None,
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
            div.child(controls::select_menu_items(
                format!("{menu_id}-menu"),
                t,
                &options,
                icons.as_deref(),
                selected,
                MENU_WIDTH,
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
        None,
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

/// One key of a chord, drawn as a small pill.
fn key_pill(t: Tokens, text: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(t.border)
        .bg(t.card_hover)
        .text_size(px(12.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(t.text)
        .child(text.to_owned())
}

/// Shortcut chip: the chord shown as key pills, clicking it starts a capture;
/// while a capture is pending the confirm/cancel buttons join the row.
#[allow(clippy::too_many_arguments)]
fn chord_chip(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    ids: &MenuIds,
    control: Control,
    set_id: String,
    chord: KeyChord,
) -> AnyElement {
    let target = CaptureTarget::Shortcut {
        set_id: set_id.clone(),
        control,
    };
    let (capturing, label) =
        capture::chip(state, &target, chord, tr(language, TextKey::PressKeyHint));
    let pending = capturing && state.capture_pending.is_some();

    let key_shared = shared.clone();
    let confirm_shared = shared.clone();
    let cancel_shared = shared.clone();

    let chip = div()
        .id(ids.chord_key.to_owned())
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(style::CONTROL_HEIGHT))
        .px(px(8.0))
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(if capturing { style::ACCENT } else { t.border })
        .bg(t.card)
        .when(capturing, |div| div.bg(t.accent_soft))
        .cursor_pointer()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .when(capturing && !pending, |row| {
                    row.child(
                        div()
                            .text_size(px(12.5))
                            .text_color(t.muted)
                            .child(label.clone()),
                    )
                })
                .when(!capturing || pending, |row| {
                    row.children(
                        label
                            .split('+')
                            .filter(|part| !part.trim().is_empty())
                            .map(|part| key_pill(t, part.trim())),
                    )
                }),
        )
        .when(!capturing, |div| {
            div.child(
                gpui_icons::lucide(gpui_icons::LucideIcon::Keyboard)
                    .size(px(15.0))
                    .text_color(t.muted),
            )
        })
        .on_click(move |_, window, cx| {
            capture::start(window, cx, &key_shared, target.clone());
        });

    div()
        .flex()
        .items_center()
        .flex_1()
        .min_w(px(0.0))
        .gap_2()
        .child(chip)
        .when(pending, |row| {
            row.child(controls::capture_buttons(
                ids.chord_key,
                t,
                move |_, _, _| capture::confirm(&confirm_shared),
                move |_, _, _| capture::cancel(&cancel_shared),
            ))
        })
        .into_any_element()
}

/// Icon picker: a dialog in the app's usual dialog style, opened from the
/// editor's icon chip. Picking an icon stores it and closes the dialog.
fn open_icon_dialog(
    window: &mut Window,
    cx: &mut App,
    t: Tokens,
    runtime: &RuntimeHandle,
    set_id: &str,
    language: Language,
) {
    let runtime = runtime.clone();
    let set_id = set_id.to_owned();
    window.open_dialog(cx, move |dialog, _, _| {
        // The builder may run more than once, so every value is read fresh.
        let current = runtime
            .config()
            .action_sets
            .iter()
            .find(|set| set.id == set_id)
            .map(|set| set.icon);
        let cells = SetIcon::ALL
            .iter()
            .map(|icon| {
                let icon_runtime = runtime.clone();
                let icon_id = set_id.clone();
                let icon = *icon;
                let is_selected = current == Some(icon);
                div()
                    .id(format!("set-icon-{}", iconset::name(icon)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_full()
                    .h(px(40.0))
                    .rounded(px(style::RADIUS_CONTROL))
                    .border_1()
                    .border_color(if is_selected { style::ACCENT } else { t.border })
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
                        20.0,
                        if is_selected { style::ACCENT } else { t.text },
                    ))
                    .on_click(move |_, window, cx| {
                        with_set(&icon_runtime, &icon_id, move |set| set.icon = icon);
                        window.close_all_dialogs(cx);
                    })
            })
            .collect::<Vec<_>>();

        dialog
            .w(px(620.0))
            .close_button(true)
            .title(
                div().flex().flex_col().gap(px(2.0)).child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(15.0))
                        .child(tr(language, TextKey::IconPickerTitle)),
                ),
            )
            .child(
                div()
                    .w_full()
                    .grid()
                    .grid_cols(10)
                    .gap(px(6.0))
                    .children(cells),
            )
    });
}

// ---------------------------------------------------------------------------
// Pure helpers.
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

/// Moves one set by a place, wrapping around both ends, and selects it.
fn move_set(runtime: &RuntimeHandle, shared: &Shared, set_id: &str, delta: i64) {
    let mut sets = runtime.config().action_sets.clone();
    let active = runtime.config().active_set.clone();
    let index = sets.iter().position(|set| set.id == set_id);
    if let Some(index) = index {
        let len = sets.len() as i64;
        let target = (index as i64 + delta).rem_euclid(len) as usize;
        let set = sets.remove(index);
        sets.insert(target, set);
        commit_sets(runtime, sets, active);
    }
    let mut shell = shared.borrow_mut();
    shell.page.selected_set = Some(set_id.to_owned());
    shell.page.open_menu = None;
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

    /// Leading icon for the action kind selector.
    fn icon(self) -> gpui_icons::LucideIcon {
        match self {
            Self::None => gpui_icons::LucideIcon::Ban,
            Self::Volume => gpui_icons::LucideIcon::Volume2,
            Self::Mute => gpui_icons::LucideIcon::VolumeX,
            Self::Brightness => gpui_icons::LucideIcon::Sun,
            Self::Scroll => gpui_icons::LucideIcon::ArrowUpDown,
            Self::Macro => gpui_icons::LucideIcon::ListChecks,
            Self::Shortcut => gpui_icons::LucideIcon::Keyboard,
            Self::Mouse => gpui_icons::LucideIcon::MousePointer,
            Self::Media => gpui_icons::LucideIcon::Play,
            Self::Launch => gpui_icons::LucideIcon::Rocket,
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

/// Static wheel preview above the set list: the geometry uses the full box.
/// Selection and reordering happen through the list, never on the wheel.
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
