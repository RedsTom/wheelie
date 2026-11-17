//! About page.

use crate::core::{APP_NAME, Language, PlatformInfo, TextKey, VERSION, tr};
use crate::ui::{theme, widgets};
use eframe::egui;

pub fn show(ui: &mut egui::Ui, language: Language, platform: PlatformInfo) {
    widgets::page_header(
        ui,
        tr(language, TextKey::AboutTitle),
        tr(language, TextKey::AboutSubtitle),
    );

    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::wheel_logo(ui, 64.0, theme::ACCENT);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(APP_NAME)
                        .font(theme::heading_font(24.0))
                        .color(theme::card_text(ui)),
                );
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(tr(language, TextKey::AppTagline))
                        .size(13.5)
                        .color(theme::muted(ui)),
                );
                ui.add_space(8.0);
                widgets::status_pill(ui, tr(language, TextKey::FreeSoftware), theme::ACCENT);
            });
        });

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(tr(language, TextKey::AboutBody))
                .size(13.5)
                .color(theme::muted(ui)),
        );

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(12.0);

        info_row(ui, language, TextKey::VersionLabel, VERSION);
        info_row(ui, language, TextKey::LicenseLabel, "GPL-3.0-only");
    });

    ui.add_space(14.0);
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(language, TextKey::PlatformCardTitle), None);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            widgets::status_pill(ui, platform.os, theme::ACCENT);
        });

        if platform.capture_is_limited {
            ui.add_space(10.0);
            ui.colored_label(
                theme::WARNING,
                tr(language, TextKey::PlatformLimitedWarning),
            );
        }
    });
}

fn info_row(ui: &mut egui::Ui, language: Language, label: TextKey, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(language, label))
                .size(13.0)
                .color(theme::muted(ui)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .size(13.0)
                    .color(theme::card_text(ui)),
            );
        });
    });
}
