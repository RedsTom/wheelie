//! Devices page: map the three knob controls to key chords.

use crate::core::{Bindings, Control, KeyChord, KeyCode, Language, TextKey, tr};
use crate::ui::controls;
use crate::ui::settings::active_profile_index;
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{Context, Window, div, px};

/// Stored in the page's open-menu slot while a key pick that would duplicate
/// another binding has been rejected; renders the distinct-keys warning until
/// the next menu interaction or a successful edit.
const CONFLICT: &str = "bindings-conflict";

pub(crate) fn render(
    state: &ShellState,
    shared: &Shared,
    _window: &Window,
    _cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let t = style::tokens(state.preferences.theme_mode);
    let language = state.preferences.language;

    let mut page = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(controls::page_header(
            t,
            tr(language, TextKey::TabDevices),
            tr(language, TextKey::DevicesSubtitle),
        ));

    if let Some(notice) = super::platform_notice_text(language, state.platform) {
        page = page.child(super::warning(notice));
    }

    let config = state.runtime.config();
    if let Some(index) = active_profile_index(&config) {
        let bindings = config.profiles[index].bindings;
        page = page.child(bindings_card(state, shared, t, language, bindings));
        if state.testing.is_some() {
            page = page.child(test_card(state, t, language));
        }
    }

    page
}

fn bindings_card(
    state: &ShellState,
    shared: &Shared,
    t: Tokens,
    language: Language,
    bindings: Bindings,
) -> impl IntoElement {
    let conflict = state.page.open_menu.as_deref() == Some(CONFLICT);
    let invalid = bindings.validate().is_err();

    controls::card(
        t,
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
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
                    ctrl: "mod-left-control",
                    alt: "mod-left-alt",
                    shift: "mod-left-shift",
                    meta: "mod-left-meta",
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
                    ctrl: "mod-press-control",
                    alt: "mod-press-alt",
                    shift: "mod-press-shift",
                    meta: "mod-press-meta",
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
                    ctrl: "mod-right-control",
                    alt: "mod-right-alt",
                    shift: "mod-right-shift",
                    meta: "mod-right-meta",
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
    ctrl: &'static str,
    alt: &'static str,
    shift: &'static str,
    meta: &'static str,
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
    let open = state.page.open_menu.as_deref() == Some(ids.menu);
    let testing = state.testing == Some(control);
    let try_label = if testing {
        tr(language, TextKey::StopTryButton)
    } else {
        tr(language, TextKey::TryButton)
    };

    let key_options = KeyCode::ALL
        .iter()
        .map(|key| key.as_str().to_owned())
        .collect::<Vec<_>>();
    let key_selected = KeyCode::ALL
        .iter()
        .position(|key| *key == chord.key)
        .unwrap_or(0);

    let menu_shared = shared.clone();
    let pick_shared = shared.clone();
    let try_shared = shared.clone();

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
        .child(
            div()
                .relative()
                .child(controls::select_button(
                    ids.menu,
                    t,
                    chord.key.as_str(),
                    open,
                    120.0,
                    move |_, _, _| {
                        let mut shell = menu_shared.borrow_mut();
                        shell.page.open_menu = if shell.page.open_menu.as_deref() == Some(ids.menu)
                        {
                            None
                        } else {
                            Some(ids.menu.to_owned())
                        };
                    },
                ))
                .when(open, |node| {
                    // `select_menu` defers its own painting (shared fix).
                    node.child(controls::select_menu(
                        ids.menu,
                        t,
                        &key_options,
                        key_selected,
                        move |index, _, _| {
                            edit_chord(&pick_shared, control, true, |chord| {
                                chord.key = KeyCode::ALL[index];
                            });
                        },
                    ))
                }),
        )
        .child(modifier_toggle(
            shared,
            t,
            ids.ctrl,
            tr(language, TextKey::ModifierControl),
            chord.modifiers.control,
            control,
            Modifier::Control,
        ))
        .child(modifier_toggle(
            shared,
            t,
            ids.alt,
            tr(language, TextKey::ModifierAlt),
            chord.modifiers.alt,
            control,
            Modifier::Alt,
        ))
        .child(modifier_toggle(
            shared,
            t,
            ids.shift,
            tr(language, TextKey::ModifierShift),
            chord.modifiers.shift,
            control,
            Modifier::Shift,
        ))
        .child(modifier_toggle(
            shared,
            t,
            ids.meta,
            tr(language, TextKey::ModifierMeta),
            chord.modifiers.meta,
            control,
            Modifier::Meta,
        ))
        .child(div().flex_1().min_w(px(0.0)))
        .child(controls::secondary_button(t, try_label, move |_, _, _| {
            let mut shell = try_shared.borrow_mut();
            if shell.testing == Some(control) {
                shell.testing = None;
            } else {
                shell.testing = Some(control);
                shell.last_key = None;
            }
            shell.hotkeys_dirty = true;
        }))
}

/// One of the four chord modifier keys.
#[derive(Clone, Copy)]
enum Modifier {
    Control,
    Alt,
    Shift,
    Meta,
}

fn modifier_toggle(
    shared: &Shared,
    t: Tokens,
    id: &'static str,
    label: &'static str,
    checked: bool,
    control: Control,
    modifier: Modifier,
) -> impl IntoElement {
    let modifier_shared = shared.clone();
    controls::toggle_labeled(id, t, label, checked, move |_, _, _| {
        edit_chord(&modifier_shared, control, false, |chord| {
            let flags = &mut chord.modifiers;
            match modifier {
                Modifier::Control => flags.control = !flags.control,
                Modifier::Alt => flags.alt = !flags.alt,
                Modifier::Shift => flags.shift = !flags.shift,
                Modifier::Meta => flags.meta = !flags.meta,
            }
        });
    })
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
    let Some(index) = active_profile_index(&config) else {
        return;
    };

    let mut chord = config.profiles[index].bindings.chord_for(control);
    update(&mut chord);
    let bindings = with_chord(config.profiles[index].bindings, control, chord);

    if bindings.validate().is_err() {
        shell.page.open_menu = Some(CONFLICT.to_owned());
        return;
    }

    match runtime.update_config(|config| config.profiles[index].bindings = bindings) {
        Ok(()) => shell.hotkeys_dirty = true,
        Err(error) => runtime.report_error(error.to_string()),
    }
    if close_menu || shell.page.open_menu.as_deref() == Some(CONFLICT) {
        shell.page.open_menu = None;
    }
}

fn test_card(state: &ShellState, t: Tokens, language: Language) -> impl IntoElement {
    let received = state
        .last_key
        .map(|chord| chord.to_string())
        .unwrap_or_else(|| tr(language, TextKey::NoKeyYet).to_owned());

    controls::card(
        t,
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(t.muted)
                    .child(tr(language, TextKey::LastKeyLabel)),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(t.text)
                    .text_right()
                    .child(received),
            ),
    )
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
