//! Shared gpui widgets used by the settings pages and the first-launch setup.
//!
//! Widgets stay free of business rules: a text field receives a [`FieldRef`]
//! built by the page that owns the target, so this module never reaches into a
//! page's layout or into the shell state.

use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, App, ClickEvent, Context, Entity, IntoElement, Render, Rgba, SharedString,
    Subscription, Window, anchored, deferred, div, px,
};
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use std::collections::HashMap;

/// Typed destination of one text field.
///
/// A page builds this at render time: `id` keeps the input state stable across
/// frames, while `write` captures the page's current selection and commits to
/// the runtime. This replaces the old string-encoded field ids and removes the
/// `controls -> pages -> controls` dependency cycle.
#[derive(Clone)]
pub struct FieldRef {
    id: SharedString,
    write: std::rc::Rc<dyn Fn(&str)>,
}

impl FieldRef {
    /// Creates a field writer for `id`. `write` receives the new text.
    pub fn new(id: impl Into<SharedString>, write: impl Fn(&str) + 'static) -> Self {
        Self {
            id: id.into(),
            write: std::rc::Rc::new(write),
        }
    }

    /// Stable identity of the field, used for the gpui element and input state.
    pub fn id(&self) -> &SharedString {
        &self.id
    }
}

/// Live text fields, keyed by [`FieldRef::id`], with their current writer.
#[derive(Default)]
pub(crate) struct FieldInputs {
    fields: HashMap<SharedString, (Entity<InputState>, FieldRef)>,
    /// Kept alive so the `InputEvent::Change` subscriptions stay installed.
    _subscriptions: Vec<Subscription>,
}

impl gpui::Global for FieldInputs {}

/// Drops every live editor. Called when the settings window opens or closes:
/// input states belong to a window, and a stale entity would keep an old
/// page's writer alive after a close-to-tray recreate.
pub fn reset_fields(cx: &mut App) {
    let global = cx.default_global::<FieldInputs>();
    global.fields.clear();
    global._subscriptions.clear();
}

/// Get or create the [`InputState`] of one field and subscribe its changes
/// to the writer carried by the [`FieldRef`].
fn ensure_field(
    field: &FieldRef,
    placeholder: SharedString,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    if let Some((entity, writer)) = cx.default_global::<FieldInputs>().fields.get_mut(&field.id) {
        // The page re-creates the ref on every frame with fresh selections.
        *writer = field.clone();
        return entity.clone();
    }
    let entity = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    let id = field.id.clone();
    let closure_id = id.clone();
    let subscription = cx.subscribe(
        &entity,
        move |entity: Entity<InputState>, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = entity.read(cx).value();
                // Re-borrow here: holding the global across `cx.new` would keep a
                // mutable borrow of the app alive while the entity is created.
                let global = cx.default_global::<FieldInputs>();
                if let Some((_, writer)) = global.fields.get(&closure_id) {
                    (writer.write)(value.as_ref());
                }
            }
        },
    );
    let global = cx.default_global::<FieldInputs>();
    global.fields.insert(id, (entity.clone(), field.clone()));
    global._subscriptions.push(subscription);
    entity
}

/// A text field backed by gpui-kit's [`InputState`].
///
/// The component owns caret, selection, clipboard and IME; the value is kept
/// in sync with `value` whenever the configuration changes (for example when
/// another set becomes selected), and typed changes go to `field`'s writer.
pub fn field_input(
    field: FieldRef,
    placeholder: impl Into<SharedString>,
    value: impl Into<SharedString>,
    width: Option<f32>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let entity = ensure_field(&field, placeholder.into(), window, cx);
    let value: SharedString = value.into();
    if entity.read(cx).value() != value {
        entity.update(cx, |state, cx| state.set_value(value.clone(), window, cx));
    }
    let field_box = match width {
        Some(width) => div().w(px(width)),
        None => div().flex_1().min_w(px(0.0)),
    };
    field_box.child(Input::new(&entity)).into_any_element()
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
                // `Label` paints with the kit foreground unless it is styled
                // directly: the parent's `text_color` would be ignored.
                .child(Label::new(title.to_owned()).text_color(t.text)),
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
///
/// `id` must be unique among the section titles of a page. `Label` derives its
/// a11y node id from the call site, so several titles rendered from here would
/// otherwise share one id and panic gpui's accessibility tree builder.
pub fn section_title(
    id: &'static str,
    t: Tokens,
    title: &str,
    subtitle: Option<&str>,
) -> impl IntoElement {
    let mut root = div().flex().flex_col().child(
        div()
            .text_size(px(16.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(Label::new(title.to_owned()).id(id).text_color(t.text)),
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
    icon_button_tinted(id, t, icon, tooltip, t.text, on_click)
}

/// Compact borderless action whose glyph carries `tone`; pass a transparent
/// tone to keep the button mounted (reserving space) but invisible.
pub fn ghost_icon_button(
    id: impl Into<SharedString>,
    icon: gpui_icons::LucideIcon,
    tooltip: &str,
    tone: Rgba,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id: SharedString = id.into();
    Button::new(id)
        .ghost()
        .compact()
        .tooltip(tooltip.to_owned())
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(18.0))
                .child(gpui_icons::lucide(icon).size(px(16.0)).text_color(tone)),
        )
}

/// Full-width dashed placeholder button with a leading plus icon, used to
/// create the next item of a list.
pub fn dashed_button(
    id: impl Into<SharedString>,
    t: Tokens,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id: SharedString = id.into();
    div()
        .id(id)
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .py(px(11.0))
        .rounded(px(style::RADIUS_CARD))
        .border_1()
        .border_dashed()
        .border_color(t.border)
        .text_size(px(13.0))
        .text_color(t.muted)
        .cursor_pointer()
        .hover(|div| div.bg(t.card_hover).text_color(t.text))
        .child(
            gpui_icons::lucide(gpui_icons::LucideIcon::Plus)
                .size(px(15.0))
                .text_color(t.muted),
        )
        .child(label.to_owned())
        .on_click(on_click)
}

/// Compact icon-only outlined action with a tinted glyph (e.g. danger).
pub fn icon_button_tinted(
    id: impl Into<SharedString>,
    _t: Tokens,
    icon: gpui_icons::LucideIcon,
    tooltip: &str,
    color: Rgba,
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
                .child(gpui_icons::lucide(icon).size(px(16.0)).text_color(color)),
        )
}

/// Status pill with the same control height as buttons.
///
/// `color` tints the wash; the ink is an opaque, AA-contrasting derivative so
/// the label stays readable in both themes.
pub fn status_pill(label: &str, color: Rgba, mode: crate::core::ThemeMode) -> impl IntoElement {
    let ink = style::on_surface(color, mode);
    div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(style::CONTROL_HEIGHT))
        .px_4()
        .rounded(px(style::RADIUS_CONTROL))
        .bg(style::alpha(color, 0.16))
        .text_size(px(14.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(ink)
        .child(label.to_owned())
}

/// Compact inline tag for tight rows.
pub fn inline_pill(label: &str, color: Rgba, mode: crate::core::ThemeMode) -> impl IntoElement {
    Tag::new()
        .rounded_full()
        .text_xs()
        .text_color(style::on_surface(color, mode))
        .bg(style::alpha(color, 0.16))
        .child(label.to_owned())
}

/// Centered empty state for a section with nothing to show yet.
pub fn empty_state(
    t: Tokens,
    title: &str,
    description: &str,
    badge: &str,
    badge_color: Rgba,
    mode: crate::core::ThemeMode,
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
                .child(Label::new(title.to_owned()).text_color(t.text)),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(t.muted)
                .child(description.to_owned()),
        )
        .child(
            div()
                .mt(px(6.0))
                .child(status_pill(badge, badge_color, mode)),
        )
}

/// Numbered onboarding step row.
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
                .bg(style::alpha(circle, 0.16))
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
                        .child(Label::new(title.to_owned()).text_color(t.text)),
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
///
/// `id` must be unique among the rows of a page; see [`section_title`] for why
/// the explicit id is required.
pub fn setting_row(
    id: &'static str,
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
                        .child(Label::new(title.to_owned()).id(id).text_color(t.text)),
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
    select_button_with_icon(id, t, None, value, open, Some(width), on_click)
}

/// Dropdown button with an optional leading icon before the value.
/// `width` sizes the button; `None` makes it fill the remaining row space.
pub fn select_button_with_icon(
    id: &'static str,
    t: Tokens,
    icon: Option<gpui_icons::LucideIcon>,
    value: &str,
    open: bool,
    width: Option<f32>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
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
        .when_some(width, |this, width| this.w(px(width)))
        .when(width.is_none(), |this| this.flex_1().min_w(px(0.0)))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .when_some(icon, |this, icon| {
                    this.child(gpui_icons::lucide(icon).size(px(15.0)).text_color(t.muted))
                })
                .child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(value.to_owned()),
                ),
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
    width: f32,
    on_pick: impl Fn(usize, &mut Window, &mut App) + Clone + 'static,
) -> impl IntoElement {
    select_menu_items(id, t, options, None, selected, width, on_pick)
}

/// Dropdown menu whose items can carry a leading icon (one per option).
///
/// The menu is deferred so it paints above its siblings, and anchored so it
/// flips above the button (and snaps back inside the window) when there is not
/// enough room below it. `width` must match the button that opens the menu.
#[allow(clippy::too_many_arguments)]
pub fn select_menu_items(
    id: impl AsRef<str>,
    t: Tokens,
    options: &[String],
    icons: Option<&[gpui_icons::LucideIcon]>,
    selected: usize,
    width: f32,
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
                        .flex()
                        .items_center()
                        .gap_2()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .when_some(
                            icons.and_then(|icons| icons.get(index).copied()),
                            |this, icon| {
                                this.child(
                                    gpui_icons::lucide(icon).size(px(14.0)).text_color(t.muted),
                                )
                            },
                        )
                        .child(
                            div()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_size(px(13.0))
                                .child(label),
                        ),
                )
                .when(is_selected, |div| {
                    div.child(
                        gpui_icons::lucide(gpui_icons::LucideIcon::Check)
                            .size(px(14.0))
                            .text_color(style::ACCENT),
                    )
                })
                .on_click(move |_event, window, cx| {
                    // A menu floats above the page: never let the click
                    // reach the controls painted underneath it.
                    cx.stop_propagation();
                    on_pick(index, window, cx)
                })
        })
        .collect::<Vec<_>>();

    deferred(
        div()
            .id(format!("{id}-menu-anchor"))
            .absolute()
            .top(px(style::CONTROL_HEIGHT + 4.0))
            .left(px(0.0))
            .w(px(width))
            .child(
                anchored().anchor(Anchor::TopLeft).child(
                    div()
                        .id(format!("{id}-menu"))
                        .w(px(width))
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
                        // Clicks on the panel margins or the scrollbar must
                        // not fall through to the widgets below either.
                        .on_click(|_, _, cx| cx.stop_propagation()),
                ),
            ),
    )
}

/// Bounded number editor: minus button, text field, plus button.
///
/// `input_width` sizes the text field; `None` stretches the whole control to
/// the remaining row space (used by the actions page).
pub fn number_field(
    field: FieldRef,
    value: impl Into<SharedString>,
    on_delta: impl Fn(i64, &mut Window, &mut App) + Clone + 'static,
    input_width: Option<f32>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id = field.id().clone();
    let minus = on_delta.clone();
    let plus = on_delta;
    div()
        .flex()
        .items_center()
        .gap_1()
        .when(input_width.is_none(), |this| this.flex_1().min_w(px(0.0)))
        .child(stepper_button(
            format!("{id}-minus"),
            gpui_icons::LucideIcon::Minus,
            move |window, cx| minus(-1, window, cx),
        ))
        .child(field_input(field, "", value, input_width, window, cx))
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
            .bg(style::TOOLTIP_BG)
            .text_size(px(12.0))
            .text_color(style::TOOLTIP_FG)
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
