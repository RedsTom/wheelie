//! Startup release notice: a modal that announces a newer published version
//! and directs the user to the GitHub page for that release.

use crate::core::{Language, Release, TextKey, tr};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, App, ClickEvent, Entity, Hsla, Rgba, SharedString, Window, div, px};

/// Backdrop and card announcing `release`.
pub(crate) fn modal(
    release: &Release,
    t: Tokens,
    language: Language,
    shared: &Shared,
    view: &Entity<SettingsView>,
) -> AnyElement {
    let tag = format!("Wheelie {}", release.tag);
    let title = tr(language, TextKey::UpdateAvailableTitle);
    let body = tr(language, TextKey::UpdateAvailableBody);

    let overlay: Hsla = Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.45,
    }
    .into();

    div()
        .id("update-modal")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(overlay)
        .child(
            div()
                .id("update-card")
                .w(px(430.0))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .rounded(px(style::RADIUS_CARD))
                .border_1()
                .border_color(t.border)
                .bg(t.card)
                .px_6()
                .py_5()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            gpui_icons::lucide(gpui_icons::LucideIcon::Rocket)
                                .size(px(18.0))
                                .text_color(style::ACCENT),
                        )
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_size(px(16.0))
                                .text_color(t.text)
                                .child(title),
                        ),
                )
                .child(
                    div()
                        .text_size(px(13.5))
                        .text_color(style::ACCENT)
                        .child(tag),
                )
                .child(div().text_size(px(12.5)).text_color(t.muted).child(body))
                .child(
                    div()
                        .mt(px(6.0))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(open_button(release, shared, view, language))
                        .child(ignore_button(release, t, shared, view, language))
                        .child(later_button(t, shared, view, language)),
                ),
        )
        .into_any_element()
}

/// Dismisses the notice and repaints the settings window.
fn dismiss_handler(
    shared: &Shared,
    view: &Entity<SettingsView>,
) -> impl Fn(&mut App) + Clone + 'static {
    let shared = shared.clone();
    let view = view.clone();
    move |cx: &mut App| {
        shared.borrow_mut().update_notice = None;
        view.update(cx, |_, cx| cx.notify());
    }
}

/// Opens the GitHub page for the release and dismisses the notice.
fn open_button(
    release: &Release,
    shared: &Shared,
    view: &Entity<SettingsView>,
    language: Language,
) -> AnyElement {
    let url = SharedString::from(release.url.clone());
    let dismiss = dismiss_handler(shared, view);
    controls::primary_button(tr(language, TextKey::UpdateOpenAction), move |_, _, cx| {
        cx.open_url(url.as_str());
        dismiss(cx);
    })
    .into_any_element()
}

/// Ignores this release from now on, then dismisses the notice.
fn ignore_button(
    release: &Release,
    t: Tokens,
    shared: &Shared,
    view: &Entity<SettingsView>,
    language: Language,
) -> AnyElement {
    let tag = release.tag.clone();
    let runtime = shared.borrow().runtime.clone();
    let dismiss = dismiss_handler(shared, view);
    controls::secondary_button(
        t,
        tr(language, TextKey::UpdateIgnoreAction),
        move |_, _, cx| {
            if let Err(error) = runtime.update_config(|config| {
                config.general.ignored_update = Some(tag.clone());
            }) {
                runtime.report_error(error.to_string());
            }
            dismiss(cx);
        },
    )
    .into_any_element()
}

/// Plain "later" affordance: dismisses the notice for this run only.
fn later_button(
    t: Tokens,
    shared: &Shared,
    view: &Entity<SettingsView>,
    language: Language,
) -> AnyElement {
    let dismiss = dismiss_handler(shared, view);
    div()
        .id("update-later")
        .cursor_pointer()
        .px_2()
        .py(px(5.0))
        .rounded(px(style::RADIUS_CONTROL))
        .text_size(px(12.5))
        .text_color(t.muted)
        .hover(|node| node.bg(t.card_hover))
        .child(tr(language, TextKey::UpdateLaterAction))
        .on_click(move |_: &ClickEvent, _: &mut Window, cx| dismiss(cx))
        .into_any_element()
}
