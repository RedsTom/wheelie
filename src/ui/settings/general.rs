//! General settings page.

use crate::app::{AppCommand, RuntimeHandle, RuntimeSnapshot, RuntimeState};
use crate::core::{Language, TextKey, tr};
use eframe::egui;

use crate::ui::{theme, widgets};

pub fn show(
    ui: &mut egui::Ui,
    language: Language,
    runtime: &RuntimeHandle,
    snapshot: &RuntimeSnapshot,
) {
    ui.heading(tr(language, TextKey::TabGeneral));
    ui.add_space(8.0);

    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr(language, TextKey::RuntimeCardTitle)).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (key, color) = match snapshot.state {
                    RuntimeState::Starting => (TextKey::StateStarting, theme::WARNING),
                    RuntimeState::Running => (TextKey::StateRunning, theme::ACCENT),
                    RuntimeState::Paused => (TextKey::StatePaused, theme::WARNING),
                    RuntimeState::Stopped => (TextKey::StateStopped, theme::WARNING),
                };
                ui.label(egui::RichText::new(tr(language, key)).color(color).strong());
            });
        });

        ui.add_space(6.0);
        let capture_key = if snapshot.capture_active {
            TextKey::CaptureActive
        } else {
            TextKey::CaptureInactive
        };
        ui.label(tr(language, capture_key));

        if let Some(profile) = &snapshot.active_profile {
            ui.label(profile);
        }

        if let Some(error) = &snapshot.last_error {
            ui.colored_label(
                egui::Color32::from_rgb(239, 68, 68),
                format!("{}: {error}", tr(language, TextKey::LastErrorLabel)),
            );
        }

        ui.add_space(8.0);
        let (label, command) = match snapshot.state {
            RuntimeState::Paused => (TextKey::ResumeAction, AppCommand::Resume),
            _ => (TextKey::PauseAction, AppCommand::Pause),
        };
        let enabled = !matches!(snapshot.state, RuntimeState::Stopped);
        if ui
            .add_enabled(enabled, egui::Button::new(tr(language, label)))
            .clicked()
        {
            let _ = runtime.send(command);
        }
    });

    ui.add_space(12.0);
    widgets::card(ui, |ui| {
        ui.label(egui::RichText::new(tr(language, TextKey::MilestoneCardTitle)).strong());
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tr(language, TextKey::Milestone0Title))
                .color(theme::ACCENT)
                .strong(),
        );
        ui.label(tr(language, TextKey::Milestone0Body));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(tr(language, TextKey::NextSteps)).strong());
        for key in [
            TextKey::Milestone1,
            TextKey::Milestone2,
            TextKey::Milestone3,
        ] {
            ui.label(format!("• {}", tr(language, key)));
        }
    });
}
