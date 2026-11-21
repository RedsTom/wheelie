//! Startup release notice: a modal that announces a newer published version
//! and directs the user to the GitHub page for that release.

use crate::core::{APP_NAME, Language, Release, TextKey, tr};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, App, Entity, FocusHandle, KeyDownEvent, Role, SharedString, div, px};

/// Backdrop and card announcing `release`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn modal(
    release: &Release,
    t: Tokens,
    language: Language,
    shared: &Shared,
    view: &Entity<SettingsView>,
    focus: &FocusHandle,
) -> AnyElement {
    let tag = format!("{} {}", APP_NAME, release.tag);
    let title = tr(language, TextKey::UpdateAvailableTitle);
    let body = tr(language, TextKey::UpdateAvailableBody);
    let dismiss = dismiss_handler(shared, view);
    let escape_dismiss = dismiss.clone();

    div()
        .id("update-modal")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(style::MODAL_SCRIM)
        .focusable()
        .track_focus(focus)
        .on_key_down(move |event: &KeyDownEvent, _, cx| {
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                cx.stop_propagation();
                escape_dismiss(cx);
            }
        })
        .on_click(|_, _, cx| cx.stop_propagation())
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
                .role(Role::Dialog)
                .aria_label(title)
                .on_click(|_, _, cx| cx.stop_propagation())
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
                        .child(later_button(t, language, dismiss)),
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

/// Dismisses the notice for this run only.
fn later_button(
    t: Tokens,
    language: Language,
    dismiss: impl Fn(&mut App) + Clone + 'static,
) -> AnyElement {
    controls::secondary_button(
        t,
        tr(language, TextKey::UpdateLaterAction),
        move |_, _, cx| {
            dismiss(cx);
        },
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The notice only uses the translated label and the published tag; no
    /// developer vocabulary or hand-written prose reaches the modal.
    #[test]
    fn notice_tag_is_built_from_app_name_and_release_tag() {
        let release = Release::new("v0.4.2", "https://example.invalid/v0.4.2");
        let tag = format!("{} {}", APP_NAME, release.tag);
        assert_eq!(tag, "Wheelie v0.4.2");
    }
}
