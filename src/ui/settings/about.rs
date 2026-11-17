//! About page.

use crate::core::{CaptureMode, Language, PlatformInfo, REPOSITORY_URL, TextKey, VERSION, tr};
use eframe::egui;

use crate::ui::{theme, widgets};

pub fn show(ui: &mut egui::Ui, language: Language, platform: PlatformInfo) {
    ui.heading(tr(language, TextKey::AboutTitle));
    ui.add_space(8.0);

    widgets::card(ui, |ui| {
        ui.label(tr(language, TextKey::AboutBody));
        ui.add_space(8.0);
        info_row(ui, tr(language, TextKey::VersionLabel), VERSION);
        info_row(ui, tr(language, TextKey::LicenseLabel), "GPL-3.0-only");
        ui.horizontal(|ui| {
            ui.label(tr(language, TextKey::RepositoryLabel));
            let _ = ui.hyperlink(REPOSITORY_URL);
        });
        ui.horizontal(|ui| {
            ui.label(tr(language, TextKey::ArchitectureLabel));
            ui.label(tr(language, TextKey::ArchitectureBody));
        });
    });

    ui.add_space(12.0);
    widgets::card(ui, |ui| {
        ui.label(egui::RichText::new(tr(language, TextKey::PlatformCardTitle)).strong());
        ui.add_space(4.0);
        ui.label(format!(
            "{} — {}",
            platform.os,
            capture_mode_label(language, platform.capture)
        ));
        if platform.capture_is_limited {
            ui.colored_label(
                theme::WARNING,
                tr(language, TextKey::PlatformLimitedWarning),
            );
        }
    });
}

fn capture_mode_label(language: Language, mode: CaptureMode) -> &'static str {
    match mode {
        CaptureMode::ExclusiveHotkeys => tr(language, TextKey::CaptureModeExclusive),
        CaptureMode::PermissionedHotkeys => tr(language, TextKey::CaptureModePermissioned),
        CaptureMode::X11BestEffort => tr(language, TextKey::CaptureModeX11),
        CaptureMode::Unsupported => tr(language, TextKey::CaptureModeUnsupported),
    }
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.monospace(value);
    });
}
