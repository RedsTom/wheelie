//! About page.

use crate::core::{APP_NAME, Language, TextKey, VERSION, tr};
use crate::ui::settings::{PageContext, platform_notice};
use crate::ui::{theme, widgets};
use eframe::egui;

pub fn show(ui: &mut egui::Ui, context: &mut PageContext<'_>) {
    let language = context.language;
    widgets::page_header(
        ui,
        tr(language, TextKey::AboutTitle),
        tr(language, TextKey::AboutSubtitle),
    );

    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            widgets::logo_image(ui, context.logo, 56.0);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(APP_NAME)
                        .font(theme::heading_font(20.0))
                        .color(theme::card_text(ui)),
                );
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(tr(language, TextKey::AppTagline))
                        .size(12.5)
                        .color(theme::muted(ui)),
                );
                ui.add_space(6.0);
                widgets::status_pill(ui, tr(language, TextKey::FreeSoftware), theme::ACCENT);
            });
        });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(tr(language, TextKey::AboutBody))
                .size(13.0)
                .color(theme::muted(ui)),
        );

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        info_row(ui, language, TextKey::VersionLabel, VERSION);
        info_row(ui, language, TextKey::LicenseLabel, "GPL-3.0-only");
        info_row(
            ui,
            language,
            TextKey::SettingsFileLabel,
            &context.runtime.config_path().display().to_string(),
        );
    });

    ui.add_space(14.0);
    widgets::card(ui, |ui| {
        widgets::section_title(ui, tr(language, TextKey::PlatformCardTitle), None);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            widgets::status_pill(ui, context.platform.os, theme::ACCENT);
        });

        ui.add_space(8.0);
        platform_notice(ui, language, context.platform);
    });
}

fn info_row(ui: &mut egui::Ui, language: Language, label: TextKey, value: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(language, label))
                .size(12.5)
                .color(theme::muted(ui)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .size(12.5)
                    .color(theme::card_text(ui)),
            );
        });
    });
}
