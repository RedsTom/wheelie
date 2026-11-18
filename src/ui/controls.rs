//! Shared gpui controls for the settings pages.
//!
//! Redesign port of the eframe `widgets.rs` vocabulary: cards, headers,
//! buttons, pills, rows, toggles, a dropdown select and a minimal text field.
//! Colors come from [`style::Tokens`]; every string arrives from the caller
//! through `TextKey` translations.

use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{App, ClickEvent, FocusHandle, KeyDownEvent, Rgba, Window, div, px};

/// Edit operations emitted by [`text_field`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// The field received focus; the reducer arms "replace on first input".
    Focused,
    /// Printable characters were typed.
    Typing(String),
    /// Backspace was pressed.
    Backspace,
    /// Paste was requested (clipboard contents).
    Paste(String),
}

/// Applies an [`Edit`] to a buffer with select-all-on-first-input behavior.
///
/// `fresh` is true after [`Edit::Focused`]: the next printable input replaces
/// the whole buffer, which stands in for caret-less editing.
pub fn apply_edit(text: &mut String, fresh: &mut bool, edit: Edit) {
    match edit {
        Edit::Focused => *fresh = true,
        Edit::Typing(chars) => {
            if *fresh {
                text.clear();
                *fresh = false;
            }
            text.push_str(&chars);
        }
        Edit::Paste(content) => {
            *text = content;
            *fresh = false;
        }
        Edit::Backspace => {
            if !*fresh {
                text.pop();
            }
        }
    }
}

/// Card container used by settings pages.
pub fn card(t: Tokens, content: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .rounded(px(style::RADIUS_CARD))
        .border_1()
        .border_color(t.border)
        .bg(t.card)
        .p(px(14.0))
        .child(content)
}

/// Compact page title and muted subtitle.
pub fn page_header(t: Tokens, title: &str, subtitle: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mb(px(14.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(24.0))
                .text_color(t.text)
                .child(title.to_owned()),
        )
        .child(
            div()
                .mt(px(3.0))
                .text_size(px(13.0))
                .text_color(t.muted)
                .child(subtitle.to_owned()),
        )
}

/// Section title used inside a card.
pub fn section_title(t: Tokens, title: &str, subtitle: Option<&str>) -> impl IntoElement {
    let mut root = div().flex().flex_col().child(
        div()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_size(px(16.0))
            .text_color(t.text)
            .child(title.to_owned()),
    );
    if let Some(subtitle) = subtitle {
        root = root.child(
            div()
                .mt(px(2.0))
                .text_size(px(12.5))
                .text_color(t.muted)
                .child(subtitle.to_owned()),
        );
    }
    root
}

/// Filled primary action. Height matches status pills.
pub fn primary_button(
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(format!("primary-{label}"))
        .flex()
        .items_center()
        .justify_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_4()
        .rounded(px(style::RADIUS_CONTROL))
        .bg(style::ACCENT)
        .text_size(px(14.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(gpui::rgb(0xFFFFFF))
        .cursor_pointer()
        .hover(|this| this.bg(gpui::rgb(0x3A79E8)))
        .active(|this| this.bg(gpui::rgb(0x2F66CC)))
        .child(label.to_owned())
        .on_click(on_click)
}

/// Subtle outlined action. Height matches status pills.
pub fn secondary_button(
    t: Tokens,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(format!("secondary-{label}"))
        .flex()
        .items_center()
        .justify_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_4()
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(t.border)
        .text_size(px(13.5))
        .text_color(t.text)
        .cursor_pointer()
        .hover(|this| this.bg(t.card_hover))
        .child(label.to_owned())
        .on_click(on_click)
}

/// Muted action in the shape of a primary button, for disabled states.
pub fn disabled_button(t: Tokens, label: &str) -> impl IntoElement {
    div()
        .id(format!("disabled-{label}"))
        .flex()
        .items_center()
        .justify_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_4()
        .rounded(px(style::RADIUS_CONTROL))
        .text_size(px(14.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(t.muted)
        .child(label.to_owned())
}

/// Status pill with the same control height as buttons.
pub fn status_pill(label: &str, color: Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_4()
        .rounded(px(style::RADIUS_CONTROL))
        .bg(Rgba { a: 0.16, ..color })
        .text_size(px(14.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(color)
        .child(label.to_owned())
}

/// Compact inline pill (no full control height) for tight rows.
pub fn inline_pill(label: &str, color: Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px_2()
        .py(px(1.0))
        .rounded_full()
        .text_size(px(11.0))
        .text_color(color)
        .bg(Rgba { a: 0.16, ..color })
        .child(label.to_owned())
}

/// Centered empty state for a section with nothing to show yet.
pub fn empty_state(
    t: Tokens,
    title: &str,
    description: &str,
    badge: &str,
    badge_color: Rgba,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .py(px(24.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(18.0))
                .text_color(t.text)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(t.muted)
                .child(description.to_owned()),
        )
        .child(div().mt(px(6.0)).child(status_pill(badge, badge_color)))
}

/// Numbered roadmap row.
pub fn timeline_item(
    t: Tokens,
    index: usize,
    title: &str,
    body: &str,
    active: bool,
) -> impl IntoElement {
    let circle = if active { style::ACCENT } else { t.border };
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(26.0))
                .rounded_full()
                .bg(Rgba { a: 0.16, ..circle })
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(circle)
                .child(index.to_string()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(14.0))
                        .text_color(t.text)
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(t.muted)
                        .child(body.to_owned()),
                ),
        )
}

/// Label/description row with a control aligned to the right.
pub fn setting_row(
    t: Tokens,
    title: &str,
    description: &str,
    control: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(14.0))
                        .text_color(t.text)
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(t.muted)
                        .child(description.to_owned()),
                ),
        )
        .child(control)
}

/// Toggle switch.
pub fn toggle(
    id: impl Into<gpui::ElementId>,
    checked: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let track = if checked {
        style::ACCENT
    } else {
        Rgba {
            a: 0.35,
            ..style::ACCENT
        }
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_1()
        .cursor_pointer()
        .child(
            div()
                .w(px(40.0))
                .h(px(22.0))
                .rounded_full()
                .bg(track)
                .flex()
                .items_center()
                .px(px(2.0))
                .child(
                    div()
                        .size(px(18.0))
                        .rounded_full()
                        .bg(gpui::rgb(0xFFFFFF))
                        .map(|knob| if checked { knob.ml(px(18.0)) } else { knob }),
                ),
        )
        .on_click(on_click)
}

/// Toggle switch with a label to its left, as a full row control.
pub fn toggle_labeled(
    id: impl Into<gpui::ElementId>,
    t: Tokens,
    label: &str,
    checked: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_3()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(20.0))
                .rounded(px(style::RADIUS_CONTROL))
                .border_1()
                .border_color(if checked { style::ACCENT } else { t.border })
                .bg(if checked {
                    style::ACCENT
                } else {
                    Rgba::default()
                })
                .when(checked, |div| {
                    div.child(
                        gpui_icons::lucide(gpui_icons::LucideIcon::Check)
                            .size(px(14.0))
                            .text_color(gpui::rgb(0xFFFFFF)),
                    )
                }),
        )
        .child(
            div()
                .text_size(px(13.5))
                .text_color(t.text)
                .child(label.to_owned()),
        )
}

/// Dropdown button: shows `value`, reports toggles.
///
/// The menu itself is drawn by [`select_menu`] inside a relative wrapper.
pub fn select_button(
    id: &'static str,
    t: Tokens,
    value: &str,
    open: bool,
    width: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .w(px(width))
        .h(px(style::CONTROL_HEIGHT))
        .px_3()
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(if open { style::ACCENT } else { t.border })
        .bg(t.card)
        .text_size(px(13.5))
        .text_color(t.text)
        .cursor_pointer()
        .hover(|this| this.bg(t.card_hover))
        .child(div().flex_1().min_w(px(0.0)).child(value.to_owned()))
        .child(
            gpui_icons::lucide(if open {
                gpui_icons::LucideIcon::ChevronUp
            } else {
                gpui_icons::LucideIcon::ChevronDown
            })
            .size(px(16.0))
            .text_color(t.muted),
        )
        .on_click(on_click)
}

/// Dropdown menu of `options`; reports the picked index.
///
/// Wrap button and menu in a `relative()` container; keep the menu after the
/// button so it paints on top.
pub fn select_menu(
    id: impl AsRef<str>,
    t: Tokens,
    options: &[String],
    selected: usize,
    on_pick: impl Fn(usize, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let id = id.as_ref().to_owned();
    let items_id = id.clone();
    let items = options
        .iter()
        .enumerate()
        .map(move |(index, label)| {
            let on_pick = on_pick.clone();
            let is_selected = index == selected;
            let label = label.clone();
            div()
                .id(format!("{items_id}-{index}"))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .px_3()
                .py(px(7.0))
                .cursor_pointer()
                .when(is_selected, |div| div.bg(t.accent_soft))
                .hover(move |div| {
                    if is_selected {
                        div
                    } else {
                        div.bg(t.card_hover)
                    }
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_size(px(13.0))
                        .child(label),
                )
                .when(is_selected, |div| {
                    div.child(
                        gpui_icons::lucide(gpui_icons::LucideIcon::Check)
                            .size(px(14.0))
                            .text_color(style::ACCENT),
                    )
                })
                .on_click(move |_event, window, cx| on_pick(index, window, cx))
        })
        .collect::<Vec<_>>();

    div()
        .id(format!("{id}-menu"))
        .absolute()
        .top(px(style::CONTROL_HEIGHT + 4.0))
        .left(px(0.0))
        .w_full()
        .max_h(px(260.0))
        .overflow_scroll()
        .flex()
        .flex_col()
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(t.border)
        .bg(t.card)
        .shadow_lg()
        .children(items)
}

/// Parameters for one text field render.
pub struct Field {
    /// Stable element id.
    pub id: &'static str,
    /// Current buffer contents.
    pub text: String,
    /// Placeholder shown while the buffer is empty.
    pub placeholder: String,
    /// Focus handle owned by the page state (cloned into the element).
    pub focus: FocusHandle,
    /// Whether the field currently holds focus (from the window).
    pub focused: bool,
    /// Fixed width; `None` fills the container.
    pub width: Option<f32>,
    /// Tokens for colors.
    pub tokens: Tokens,
}

/// Minimal single-line text field: focus, typing, backspace, paste.
///
/// Editing is select-all-on-first-input (see [`apply_edit`]); no caret or
/// selection rendering in this iteration.
pub fn text_field(
    field: Field,
    on_edit: impl Fn(Edit, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let t = field.tokens;
    let click_edit = on_edit.clone();
    let key_edit = on_edit;
    let focus = field.focus.clone();
    let placeholder = field.placeholder.to_owned();
    let text = field.text.to_owned();

    div()
        .id(field.id)
        .flex()
        .items_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_3()
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(if field.focused {
            style::ACCENT
        } else {
            t.border
        })
        .bg(t.card)
        .text_size(px(13.5))
        .when_some(field.width, |div, width| div.w(px(width)))
        .when(field.width.is_none(), |div| div.w_full())
        .track_focus(&focus)
        .child(if text.is_empty() {
            div().text_color(t.muted).child(placeholder)
        } else {
            div().text_color(t.text).child(text)
        })
        .on_click(move |_, window, cx| {
            focus.focus(window, cx);
            click_edit(Edit::Focused, window, cx);
        })
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            let keystroke = &event.keystroke;
            let ctrl = keystroke.modifiers.control || keystroke.modifiers.platform;
            if ctrl && keystroke.key.eq_ignore_ascii_case("v") {
                let pasted = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .unwrap_or_default();
                key_edit(Edit::Paste(pasted), window, cx);
                return;
            }
            if keystroke.key.eq_ignore_ascii_case("backspace") {
                key_edit(Edit::Backspace, window, cx);
                return;
            }
            if ctrl || keystroke.modifiers.alt {
                return;
            }
            if let Some(chars) = keystroke.key_char.as_ref()
                && !chars.is_empty()
            {
                key_edit(Edit::Typing(chars.clone()), window, cx);
            } else if keystroke.key.chars().count() == 1 && keystroke.key_char.is_none() {
                // Fallback for layouts that leave key_char empty.
                key_edit(Edit::Typing(keystroke.key.clone()), window, cx);
            }
        })
}

/// Bounded number editor: minus button, text field, plus button.
pub fn number_field(
    field: Field,
    on_edit: impl Fn(Edit, &mut Window, &mut App) + Clone + 'static,
    on_delta: impl Fn(i64, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let minus = on_delta.clone();
    let plus = on_delta;
    let minus_id = format!("{}-minus", field.id);
    let plus_id = format!("{}-plus", field.id);
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(stepper_button(
            minus_id,
            gpui_icons::LucideIcon::Minus,
            move |window, cx| minus(-1, window, cx),
        ))
        .child(text_field(field, on_edit))
        .child(stepper_button(
            plus_id,
            gpui_icons::LucideIcon::Plus,
            move |window, cx| plus(1, window, cx),
        ))
}

fn stepper_button(
    id: String,
    icon: gpui_icons::LucideIcon,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(style::CONTROL_HEIGHT))
        .rounded(px(style::RADIUS_CONTROL))
        .border_1()
        .border_color(style::ACCENT)
        .cursor_pointer()
        .hover(|this| this.bg(style::ACCENT))
        .child(
            gpui_icons::lucide(icon)
                .size(px(16.0))
                .text_color(style::ACCENT),
        )
        .on_click(move |_, window, cx| on_click(window, cx))
}

/// Minimal text tooltip view used by [`text_tooltip`].
pub struct SimpleTooltip {
    text: String,
}

impl Render for SimpleTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py(px(3.0))
            .rounded(px(style::RADIUS_CONTROL))
            .bg(gpui::rgb(0x000000).alpha(0.85))
            .text_size(px(12.0))
            .text_color(gpui::rgb(0xFFFFFF))
            .child(self.text.clone())
    }
}

/// Builds the tooltip required by `InteractiveElement::tooltip`.
///
/// Use as `.tooltip(controls::text_tooltip(tr(language, TextKey::X)))`.
pub fn text_tooltip(
    text: impl Into<String>,
) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    let text = text.into();
    move |_, cx| {
        let owned = text.clone();
        cx.new(move |_| SimpleTooltip { text: owned }).into()
    }
}
