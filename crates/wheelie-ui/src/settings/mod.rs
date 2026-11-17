//! Settings view: navigation and page dispatch.

mod about;
mod general;

use eframe::egui;
use wheelie_app::{RuntimeHandle, RuntimeSnapshot};
use wheelie_core::{Language, PlatformInfo, TextKey, tr};

use crate::widgets;

/// Settings sections shown in the left navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    /// General runtime status and application options.
    #[default]
    General,
    /// Knob device and key bindings.
    Devices,
    /// Action catalog and parameters.
    Actions,
    /// Macro editor.
    Macros,
    /// Version, license and platform information.
    About,
}

impl SettingsTab {
    /// All tabs, in display order.
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Devices,
        Self::Actions,
        Self::Macros,
        Self::About,
    ];

    /// Translated tab label.
    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::General => tr(language, TextKey::TabGeneral),
            Self::Devices => tr(language, TextKey::TabDevices),
            Self::Actions => tr(language, TextKey::TabActions),
            Self::Macros => tr(language, TextKey::TabMacros),
            Self::About => tr(language, TextKey::TabAbout),
        }
    }
}

/// Draws the left navigation.
pub fn navigation(ui: &mut egui::Ui, language: Language, selected: &mut SettingsTab) {
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new(tr(language, TextKey::Navigation))
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    for tab in SettingsTab::ALL {
        let _ = ui.selectable_value(selected, tab, tab.label(language));
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(tr(language, TextKey::Status));
    ui.label(
        egui::RichText::new(tr(language, TextKey::Milestone0Title))
            .color(crate::theme::ACCENT)
            .strong(),
    );
}

/// Draws the selected settings page.
pub fn page(
    ui: &mut egui::Ui,
    language: Language,
    tab: SettingsTab,
    runtime: &RuntimeHandle,
    snapshot: &RuntimeSnapshot,
    platform: PlatformInfo,
) {
    match tab {
        SettingsTab::General => general::show(ui, language, runtime, snapshot),
        SettingsTab::Devices => {
            placeholder(ui, language, TextKey::PlaceholderDevices);
        }
        SettingsTab::Actions => {
            placeholder(ui, language, TextKey::PlaceholderActions);
        }
        SettingsTab::Macros => {
            placeholder(ui, language, TextKey::PlaceholderMacros);
        }
        SettingsTab::About => about::show(ui, language, platform),
    }
}

fn placeholder(ui: &mut egui::Ui, language: Language, message: TextKey) {
    widgets::card(ui, |ui| {
        ui.label(egui::RichText::new(tr(language, message)).weak());
    });
}
