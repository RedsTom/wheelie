//! Visual theme for Wheelie.

use eframe::egui;

/// Accent color used by the wheel and primary highlights.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(124, 92, 250);

/// Warning color for platform limitations.
pub const WARNING: egui::Color32 = egui::Color32::from_rgb(245, 158, 11);

/// Theme exposed in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Dark theme.
    #[default]
    Dark,
    /// Light theme.
    Light,
}

impl ThemeMode {
    /// All modes, in display order.
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];
}

/// Applies the selected theme to the egui context.
pub fn apply(context: &egui::Context, mode: ThemeMode) {
    context.set_visuals(match mode {
        ThemeMode::Dark => egui::Visuals::dark(),
        ThemeMode::Light => egui::Visuals::light(),
    });
}
