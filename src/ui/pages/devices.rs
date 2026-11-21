//! Devices page: map the three knob controls to key chords.

use crate::core::{Bindings, Control, KeyChord, Language, TextKey, tr};
use crate::ui::capture::{self, CaptureTarget};
use crate::ui::controls;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{Context, Window, div, px};

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    _window: &Window,
    _cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;

    let mut page = div()
        .id("devices-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .overflow_scroll()
        .gap(px(8.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabDevices),
            tr(language, TextKey::DevicesSubtitle),
        ));

    if let Some(notice) = super::platform_notice_text(language, state.platform) {
        page = page.child(super::warning(notice));
    }

    let config = state.runtime.config();
    if let Some(index) = config.active_profile_index() {
        let bindings = config.profiles[index].bindings;
        page = page.child(bindings_card(state, shared, t, language, bindings));
    }

    page
}

pub(crate) fn bindings_card(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    bindings: Bindings,
) -> impl IntoElement {
    let conflict = state.page.binding_conflict.is_some();
    let invalid = bindings.validate().is_err();

    controls::card(
        "device-bindings",
        t,
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(t.muted)
                    .child(tr(language, TextKey::KeyLabel)),
            )
            .child(binding_row(
                state,
                shared,
                t,
                language,
                Control::Left,
                bindings.left,
                RowIds {
                    row: "binding-left",
                    menu: "key-left",
                },
            ))
            .child(super::separator(t))
            .child(binding_row(
                state,
                shared,
                t,
                language,
                Control::Press,
                bindings.press,
                RowIds {
                    row: "binding-press",
                    menu: "key-press",
                },
            ))
            .child(super::separator(t))
            .child(binding_row(
                state,
                shared,
                t,
                language,
                Control::Right,
                bindings.right,
                RowIds {
                    row: "binding-right",
                    menu: "key-right",
                },
            ))
            .when(conflict || invalid, |node| {
                node.child(super::warning(tr(
                    language,
                    TextKey::BindingsMustBeDistinct,
                )))
            }),
    )
}

/// Element ids owned by one binding row; the row id keeps the shared Try
/// button label unique across rows through the global id path.
#[derive(Clone, Copy)]
struct RowIds {
    row: &'static str,
    menu: &'static str,
}

fn binding_row(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    control: Control,
    chord: KeyChord,
    ids: RowIds,
) -> impl IntoElement {
    let target = CaptureTarget::Binding(control);
    let (capturing, label) =
        capture::chip(state, &target, chord, tr(language, TextKey::PressKeyHint));

    let capture_shared = shared.clone();
    let confirm_shared = shared.clone();
    let cancel_shared = shared.clone();
    let outside_target = target.clone();
    let outside_shared = shared.clone();

    div()
        .id(ids.row)
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(110.0))
                .flex_none()
                .text_size(px(14.0))
                .text_color(t.text)
                .child(control_label(language, control)),
        )
        .child(div().flex_1().min_w(px(0.0)))
        .when(capturing && state.capture_pending.is_some(), |row| {
            row.child(controls::capture_buttons(
                ids.row,
                t,
                move |_, _, _| capture::confirm(&confirm_shared),
                move |_, _, _| capture::cancel(&cancel_shared),
            ))
        })
        .child(controls::key_capture_button(
            ids.menu.to_owned(),
            t,
            &label,
            capturing,
            move |_, window, cx| {
                capture::start(window, cx, &capture_shared, target.clone());
            },
        ))
        .when(capturing, |row| {
            row.on_mouse_down_out(move |_, _, _| {
                capture::abandon_if_target(&outside_shared, &outside_target);
            })
        })
}

/// Stores a key captured in the settings window as the chord of `control`.
pub(crate) fn capture_chord(shared: &Shared, control: Control, captured: KeyChord) {
    edit_chord(shared, control, true, |chord| *chord = captured);
}

/// Applies one chord edit to the active profile. Invalid candidates (a chord
/// already used by another control) are rejected and reported through the
/// distinct-keys warning instead of being persisted.
fn edit_chord(
    shared: &Shared,
    control: Control,
    close_menu: bool,
    update: impl FnOnce(&mut KeyChord),
) {
    let mut shell = shared.borrow_mut();
    let runtime = shell.runtime.clone();
    let config = runtime.config();
    let Some(index) = config.active_profile_index() else {
        return;
    };

    let mut chord = config.profiles[index].bindings.chord_for(control);
    update(&mut chord);
    let bindings = with_chord(config.profiles[index].bindings, control, chord);

    if bindings.validate().is_err() {
        shell.page.binding_conflict = Some(control);
        shell.page.open_menu = None;
        return;
    }

    if let Err(error) = runtime.update_config(|config| config.profiles[index].bindings = bindings) {
        runtime.report_error(error.to_string());
    } else {
        shell.page.binding_conflict = None;
    }
    if close_menu {
        shell.page.open_menu = None;
    }
}

fn with_chord(mut bindings: Bindings, control: Control, chord: KeyChord) -> Bindings {
    match control {
        Control::Left => bindings.left = chord,
        Control::Right => bindings.right = chord,
        Control::Press => bindings.press = chord,
    }
    bindings
}

fn control_label(language: Language, control: Control) -> &'static str {
    match control {
        Control::Left => tr(language, TextKey::ControlLeft),
        Control::Right => tr(language, TextKey::ControlRight),
        Control::Press => tr(language, TextKey::ControlPress),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::KeyCode;

    #[test]
    fn replacing_one_chord_keeps_the_others() {
        let bindings = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        };

        let updated = with_chord(bindings, Control::Press, KeyChord::new(KeyCode::F16));
        assert_eq!(updated.press, KeyChord::new(KeyCode::F16));
        assert_eq!(updated.left, bindings.left);
        assert_eq!(updated.right, bindings.right);
    }
}
