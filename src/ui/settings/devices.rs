//! Devices page: map the three knob controls to key chords.

use crate::core::{Bindings, Control, TextKey, tr};
use crate::ui::settings::{PageContext, active_profile_index, platform_notice, warning};
use crate::ui::{theme, widgets};
use eframe::egui;

pub fn show(ui: &mut egui::Ui, context: &mut PageContext<'_>) {
    let language = context.language;
    widgets::page_header(
        ui,
        tr(language, TextKey::TabDevices),
        tr(language, TextKey::DevicesSubtitle),
    );

    platform_notice(ui, language, context.platform);

    let config = context.runtime.config();
    let Some(index) = active_profile_index(&config) else {
        return;
    };
    let mut bindings = config.profiles[index].bindings;
    let mut changed = false;

    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(tr(language, TextKey::KeyLabel))
                    .size(12.5)
                    .color(theme::muted(ui)),
            );
        });
        ui.add_space(6.0);

        for control in [Control::Left, Control::Press, Control::Right] {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [110.0, theme::CONTROL_HEIGHT],
                    egui::Label::new(
                        egui::RichText::new(control_label(language, control))
                            .size(14.0)
                            .color(theme::card_text(ui)),
                    ),
                );

                let mut chord = bindings.chord_for(control);
                if widgets::chord_picker(ui, ("knob-key", control), language, &mut chord) {
                    bindings = with_chord(bindings, control, chord);
                    changed = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let testing = context.testing == Some(control);
                    let label = if testing {
                        tr(language, TextKey::StopTryButton)
                    } else {
                        tr(language, TextKey::TryButton)
                    };
                    if widgets::secondary_button(ui, label).clicked() {
                        context.test_request = Some(if testing { None } else { Some(control) });
                    }
                });
            });
            ui.add_space(4.0);
        }

        if bindings.validate().is_err() {
            ui.add_space(6.0);
            warning(ui, tr(language, TextKey::BindingsMustBeDistinct));
        }
    });

    if changed && bindings.validate().is_ok() {
        let result = context
            .runtime
            .update_config(|config| config.profiles[index].bindings = bindings);
        if let Err(error) = result {
            context.runtime.report_error(error.to_string());
        } else {
            context.bindings_changed = true;
        }
    }

    if context.testing.is_some() {
        ui.add_space(14.0);
        widgets::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(tr(language, TextKey::LastKeyLabel))
                        .size(12.5)
                        .color(theme::muted(ui)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = context
                        .last_key
                        .map(|chord| chord.to_string())
                        .unwrap_or_else(|| tr(language, TextKey::NoKeyYet).to_owned());
                    ui.label(
                        egui::RichText::new(text)
                            .size(13.0)
                            .color(theme::card_text(ui)),
                    );
                });
            });
        });
    }
}

fn with_chord(mut bindings: Bindings, control: Control, chord: crate::core::KeyChord) -> Bindings {
    match control {
        Control::Left => bindings.left = chord,
        Control::Right => bindings.right = chord,
        Control::Press => bindings.press = chord,
    }
    bindings
}

fn control_label(language: crate::core::Language, control: Control) -> &'static str {
    match control {
        Control::Left => tr(language, TextKey::ControlLeft),
        Control::Right => tr(language, TextKey::ControlRight),
        Control::Press => tr(language, TextKey::ControlPress),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{KeyChord, KeyCode};

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
