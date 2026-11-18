use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, IntoElement, Render, Rgba, SharedString,
    Subscription, Window, deferred, div, px,
};
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use std::collections::HashMap;

/// Live text fields, keyed by control id, plus the shell handle used to
/// commit typed text into the configuration.
#[derive(Default)]
pub(crate) struct FieldInputs {
    fields: HashMap<String, Entity<InputState>>,
    /// Kept alive so the `InputEvent::Change` subscriptions stay installed.
    _subscriptions: Vec<Subscription>,
    shell: Option<crate::ui::shell::Shared>,
}

impl gpui::Global for FieldInputs {}

/// Installs the shared shell state so [`field_input`] commits can reach the
/// runtime. Called once from `run`, right after the shell is built.
pub fn install_fields(cx: &mut App, shared: crate::ui::shell::Shared) {
    cx.default_global::<FieldInputs>().shell = Some(shared);
}

/// Get or create the [`InputState`] of one field and subscribe its changes
/// to the configuration commit for that field id.
fn ensure_field(
    id: &str,
    placeholder: SharedString,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    if let Some(existing) = cx.default_global::<FieldInputs>().fields.get(id).cloned() {
        return existing;
    }
    let entity = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    let subscription = cx.subscribe(&entity, {
        let id = id.to_owned();
        move |entity: Entity<InputState>, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = entity.read(cx).value();
                commit_field(&id, value.as_ref(), cx);
            }
        }
    });
    let global = cx.default_global::<FieldInputs>();
    global.fields.insert(id.to_owned(), entity.clone());
    global._subscriptions.push(subscription);
    entity
}

/// Routes a field change into the configuration.
///
/// Field ids are stable and encode their target: `set-name` and
/// `macro-name` act on the page's current selection, `path-*`, `steps-*`
/// and `amount-*` on the selected set's control, `step-N-text` and
/// `step-N-value` on the selected macro's step N.
fn commit_field(id: &str, value: &str, cx: &mut App) {
    let Some(shared) = cx.global::<FieldInputs>().shell.clone() else {
        return;
    };
    let (runtime, selected_set, selected_macro) = {
        let shell = shared.borrow();
        (
            shell.runtime.clone(),
            shell.page.selected_set.clone(),
            shell.page.selected_macro.clone(),
        )
    };
    let config = runtime.config();
    let first_set = || config.action_sets.first().map(|set| set.id.clone());
    let first_macro = || config.macros.first().map(|m| m.name.clone());

    if let Some(control) = id.strip_prefix("path-") {
        let Some(set_id) = selected_set.or_else(first_set) else {
            return;
        };
        drop(config);
        let Some(control) = control_id(control) else {
            return;
        };
        super::pages::actions::with_set_action(&runtime, &set_id, control, |action, _| {
            if let crate::core::Action::Launch { path } = action {
                *path = std::path::PathBuf::from(value);
            }
        });
        return;
    }
    for prefix in ["steps-", "amount-"] {
        if let Some(control) = id.strip_prefix(prefix) {
            let Some(set_id) = selected_set.or_else(first_set) else {
                return;
            };
            drop(config);
            let Some(control) = control_id(control) else {
                return;
            };
            let value = value.to_owned();
            super::pages::actions::with_set_action(&runtime, &set_id, control, move |action, _| {
                if let Ok(number) = value.parse::<i64>() {
                    super::pages::actions::set_numeric(action, number);
                }
            });
            return;
        }
    }
    if id == "set-name" {
        let Some(set_id) = selected_set.or_else(first_set) else {
            return;
        };
        drop(config);
        super::pages::actions::with_set(&runtime, &set_id, |set| set.name = value.to_owned());
        return;
    }
    if id == "macro-name" {
        let Some(name) = selected_macro.or_else(first_macro) else {
            return;
        };
        drop(config);
        super::pages::macros::rename_macro(&runtime, &shared, &name, value);
        return;
    }
    if let Some(rest) = id.strip_prefix("step-") {
        let (index, kind) = match rest.rsplit_once('-') {
            Some((index, kind)) => (index, kind),
            None => return,
        };
        let Ok(index) = index.parse::<usize>() else {
            return;
        };
        let Some(macro_name) = selected_macro.or_else(first_macro) else {
            return;
        };
        drop(config);
        let value = value.to_owned();
        super::pages::macros::with_macro(&runtime, &macro_name, |m| {
            let Some(step) = m.steps.get_mut(index) else {
                return false;
            };
            match kind {
                "text" => {
                    if matches!(step, crate::core::MacroStep::Text { .. }) {
                        *step = crate::core::MacroStep::Text { text: value };
                        true
                    } else {
                        false
                    }
                }
                "value" => {
                    if let Ok(number) = value.parse::<i64>() {
                        super::pages::macros::set_numeric(step, number);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        });
    }
}

/// Parses the control suffix of `path-`/`steps-`/`amount-` ids.
fn control_id(name: &str) -> Option<crate::core::Control> {
    match name {
        "left" => Some(crate::core::Control::Left),
        "press" => Some(crate::core::Control::Press),
        "right" => Some(crate::core::Control::Right),
        _ => None,
    }
}

/// A text field backed by gpui-kit's [`InputState`].
///
/// The component owns caret, selection, clipboard and IME; the value is kept
/// in sync with `value` whenever the configuration changes (for example when
/// another set becomes selected), and typed changes are committed through the
/// subscription installed in [`ensure_field`].
pub fn field_input(
    id: &str,
    placeholder: impl Into<SharedString>,
    value: impl Into<SharedString>,
    width: Option<f32>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let entity = ensure_field(id, placeholder.into(), window, cx);
    let value: SharedString = value.into();
    if entity.read(cx).value() != value {
        entity.update(cx, |state, cx| state.set_value(value.clone(), window, cx));
    }
    let field = div().when_some(width, |div, width| div.w(px(width)));
    field.child(Input::new(&entity)).into_any_element()
}

/// Card container used by settings pages, backed by gpui-kit `GroupBox`.
///
/// `id` must be unique among the cards of a page. `GroupBox` falls back to
/// `"group-box"` when no id is set, which makes sibling cards share one
/// accessibility node id and panics gpui once accessibility is activated.
pub fn card(id: &'static str, _t: Tokens, content: impl IntoElement) -> impl IntoElement {
    GroupBox::new().id(id).outline().p(px(12.0)).child(content)
}

/// Compact page title and muted subtitle.
pub fn page_header(t: Tokens, title: &str, subtitle: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mb(px(10.0))
        .child(
            div()
                .text_size(px(24.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.text)
                .child(Label::new(title.to_owned())),
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
            .text_size(px(16.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(t.text)
            .child(Label::new(title.to_owned())),
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

/// Filled primary action.
pub fn primary_button(
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(SharedString::from(format!("primary-{label}")))
        .primary()
        .label(label.to_owned())
        .on_click(on_click)
}

/// Subtle outlined action.
pub fn secondary_button(
    _t: Tokens,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(SharedString::from(format!("secondary-{label}")))
        .outline()
        .label(label.to_owned())
        .on_click(on_click)
}

/// Muted action in the shape of a primary button, for disabled states.
pub fn disabled_button(_t: Tokens, label: &str) -> impl IntoElement {
    Button::new(SharedString::from(format!("disabled-{label}")))
        .primary()
        .label(label.to_owned())
        .disabled(true)
}

/// Outlined action with a leading icon.
pub fn icon_text_button(
    id: impl Into<SharedString>,
    t: Tokens,
    icon: gpui_icons::LucideIcon,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(id.into()).outline().on_click(on_click).child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(gpui_icons::lucide(icon).size(px(16.0)).text_color(t.muted))
            .child(label.to_owned()),
    )
}

/// Destructive outlined action with a leading icon.
pub fn danger_button(
    id: impl Into<SharedString>,
    icon: gpui_icons::LucideIcon,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(id.into())
        .danger()
        .outline()
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    gpui_icons::lucide(icon)
                        .size(px(16.0))
                        .text_color(style::DANGER),
                )
                .child(label.to_owned()),
        )
}

/// Compact icon-only outlined action.
pub fn icon_button(
    id: impl Into<SharedString>,
    t: Tokens,
    icon: gpui_icons::LucideIcon,
    tooltip: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(id.into())
        .outline()
        .compact()
        .tooltip(tooltip.to_owned())
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(18.0))
                .child(gpui_icons::lucide(icon).size(px(16.0)).text_color(t.text)),
        )
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

/// Compact inline tag for tight rows.
pub fn inline_pill(label: &str, color: Rgba) -> impl IntoElement {
    Tag::new()
        .rounded_full()
        .text_xs()
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
                .child(Label::new(title.to_owned())),
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
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(14.0))
                        .text_color(t.text)
                        .child(Label::new(title.to_owned())),
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
                        .child(Label::new(title.to_owned())),
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

/// Toggle switch backed by gpui-kit `Switch`.
pub fn toggle(
    id: impl Into<gpui::ElementId>,
    checked: bool,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Switch::new(id).checked(checked).on_click(on_change)
}

/// Toggle switch with a visible label.
pub fn toggle_labeled(
    id: impl Into<gpui::ElementId>,
    _t: Tokens,
    label: &str,
    checked: bool,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Switch::new(id)
        .label(label.to_owned())
        .checked(checked)
        .on_click(on_change)
}

/// Dropdown button: shows `value`, reports toggles.
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
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(value.to_owned()),
        )
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

/// Key chip that starts a key capture.
pub fn key_capture_button(
    id: String,
    t: Tokens,
    label: &str,
    capturing: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("{id}-wrap")))
        .min_w(px(150.0))
        .rounded(px(style::RADIUS_CONTROL))
        .when(capturing, |d| d.bg(t.accent_soft))
        .child(
            Button::new(SharedString::from(id))
                .outline()
                .label(label.to_owned())
                .when(capturing, |this| this.border_color(style::ACCENT))
                .on_click(on_click),
        )
}

/// Confirm (check) and cancel (cross) buttons shown next to a pending key
/// capture.
pub fn capture_buttons(
    id: &str,
    t: Tokens,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let confirm = Button::new(SharedString::from(format!("{id}-confirm")))
        .success()
        .outline()
        .compact()
        .on_click(on_confirm)
        .child(
            div().flex().items_center().justify_center().child(
                gpui_icons::lucide(gpui_icons::LucideIcon::Check)
                    .size(px(16.0))
                    .text_color(style::SUCCESS),
            ),
        );
    let cancel = Button::new(SharedString::from(format!("{id}-cancel")))
        .outline()
        .compact()
        .on_click(on_cancel)
        .child(
            div().flex().items_center().justify_center().child(
                gpui_icons::lucide(gpui_icons::LucideIcon::X)
                    .size(px(16.0))
                    .text_color(t.muted),
            ),
        );
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(confirm)
        .child(cancel)
}

/// Dropdown menu of `options`; reports the picked index.
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

    deferred(
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
            .children(items),
    )
}

/// Bounded number editor: minus button, text field, plus button.
pub fn number_field(
    id: &str,
    value: impl Into<SharedString>,
    on_delta: impl Fn(i64, &mut Window, &mut App) + Clone + 'static,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let minus = on_delta.clone();
    let plus = on_delta;
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(stepper_button(
            format!("{id}-minus"),
            gpui_icons::LucideIcon::Minus,
            move |window, cx| minus(-1, window, cx),
        ))
        .child(field_input(id, "", value, Some(70.0), window, cx))
        .child(stepper_button(
            format!("{id}-plus"),
            gpui_icons::LucideIcon::Plus,
            move |window, cx| plus(1, window, cx),
        ))
        .into_any_element()
}

fn stepper_button(
    id: String,
    icon: gpui_icons::LucideIcon,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(SharedString::from(id))
        .outline()
        .compact()
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(
            div().flex().items_center().justify_center().child(
                gpui_icons::lucide(icon)
                    .size(px(16.0))
                    .text_color(style::ACCENT),
            ),
        )
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
