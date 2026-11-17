//! General settings page.

use crate::app::{AppCommand, RuntimeHandle, RuntimeSnapshot, RuntimeState};
use crate::core::{Language, TextKey, tr};
use crate::ui::settings::{UiPreferences, state_label};
use crate::ui::{ThemeMode, theme, widgets};
use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    preferences: &mut UiPreferences,
    runtime: &RuntimeHandle,
    snapshot: &RuntimeSnapshot,
) {
    let language = preferences.language;
    widgets::page_header(
        ui,
        tr(language, TextKey::TabGeneral),
        tr(language, TextKey::GeneralSubtitle),
    );

    runtime_card(ui, language, runtime, snapshot);
    ui.add_space(14.0);
    ui.columns(2, |columns| {
        preferences_card(&mut columns[0], preferences);
        getting_started_card(&mut columns[1], language);
    });
}

fn runtime_card(
    ui: &mut egui::Ui,
    language: Language,
    runtime: &RuntimeHandle,
    snapshot: &RuntimeSnapshot,
) {
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::wheel_logo(ui, 54.0, theme::ACCENT);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                let title = match snapshot.state {
                    RuntimeState::Running | RuntimeState::Starting => TextKey::RuntimeReadyTitle,
                    RuntimeState::Paused => TextKey::RuntimePausedTitle,
                    RuntimeState::Stopped => TextKey::RuntimeStoppedTitle,
                };
                let body = match snapshot.state {
                    RuntimeState::Running | RuntimeState::Starting => TextKey::RuntimeReadyBody,
                    RuntimeState::Paused => TextKey::RuntimePausedBody,
                    RuntimeState::Stopped => TextKey::RuntimeStoppedBody,
                };
                ui.label(
                    egui::RichText::new(tr(language, title))
                        .font(theme::heading_font(21.0))
                        .color(theme::card_text(ui)),
                );
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(tr(language, body))
                        .size(13.5)
                        .color(theme::muted(ui)),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (label, command) = match snapshot.state {
                    RuntimeState::Paused => (TextKey::ResumeAction, AppCommand::Resume),
                    _ => (TextKey::PauseAction, AppCommand::Pause),
                };
                let enabled = !matches!(snapshot.state, RuntimeState::Stopped);
                let clicked = ui
                    .add_enabled_ui(enabled, |ui| {
                        widgets::primary_button(ui, tr(language, label))
                    })
                    .inner
                    .clicked();
                if clicked {
                    let _ = runtime.send(command);
                }

                ui.add_space(4.0);
                let (state, color) = state_label(language, snapshot.state);
                widgets::status_pill(ui, state, color);
            });
        });

        if let Some(error) = &snapshot.last_error {
            ui.add_space(10.0);
            ui.colored_label(
                theme::DANGER,
                format!("{}: {error}", tr(language, TextKey::LastErrorLabel)),
            );
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(tr(language, TextKey::ActiveProfileLabel))
                    .size(12.5)
                    .color(theme::muted(ui)),
            );
            ui.label(
                egui::RichText::new(
                    snapshot
                        .active_profile
                        .as_deref()
                        .unwrap_or_else(|| tr(language, TextKey::NoProfileLabel)),
                )
                .size(12.5)
                .color(theme::card_text(ui)),
            );
        });
    });
}

fn preferences_card(ui: &mut egui::Ui, preferences: &mut UiPreferences) {
    let language = preferences.language;
    widgets::card(ui, |ui| {
        widgets::section_title(
            ui,
            tr(language, TextKey::PreferencesTitle),
            Some(tr(language, TextKey::PreferencesBody)),
        );
        ui.add_space(14.0);

        widgets::setting_row(
            ui,
            tr(language, TextKey::ThemeLabel),
            tr(language, TextKey::ThemeDescription),
            |ui| {
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(theme_label(preferences.language, preferences.theme_mode))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for mode in ThemeMode::ALL {
                            ui.selectable_value(
                                &mut preferences.theme_mode,
                                mode,
                                theme_label(preferences.language, mode),
                            );
                        }
                    });
            },
        );

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        widgets::setting_row(
            ui,
            tr(language, TextKey::LanguageLabel),
            tr(language, TextKey::LanguageDescription),
            |ui| {
                egui::ComboBox::from_id_salt("language")
                    .selected_text(preferences.language.native_name())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for language in Language::ALL {
                            ui.selectable_value(
                                &mut preferences.language,
                                language,
                                language.native_name(),
                            );
                        }
                    });
            },
        );
    });
}

fn getting_started_card(ui: &mut egui::Ui, language: Language) {
    widgets::card(ui, |ui| {
        widgets::section_title(
            ui,
            tr(language, TextKey::GettingStartedTitle),
            Some(tr(language, TextKey::GettingStartedSubtitle)),
        );
        ui.add_space(14.0);
        widgets::timeline_item(
            ui,
            1,
            tr(language, TextKey::StepConfigureTitle),
            tr(language, TextKey::StepConfigureBody),
            true,
        );
        ui.add_space(10.0);
        widgets::timeline_item(
            ui,
            2,
            tr(language, TextKey::StepAssignTitle),
            tr(language, TextKey::StepAssignBody),
            false,
        );
        ui.add_space(10.0);
        widgets::timeline_item(
            ui,
            3,
            tr(language, TextKey::StepOpenTitle),
            tr(language, TextKey::StepOpenBody),
            false,
        );
    });
}

fn theme_label(language: Language, mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Dark => tr(language, TextKey::DarkTheme),
        ThemeMode::Light => tr(language, TextKey::LightTheme),
    }
}
