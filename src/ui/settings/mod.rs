//! Settings view: navigation, preferences and page dispatch.

mod about;
mod general;

use crate::app::{RuntimeHandle, RuntimeSnapshot, RuntimeState};
use crate::core::{Language, PlatformInfo, TextKey, tr};
use crate::ui::{ThemeMode, theme, widgets};
use eframe::egui;

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

/// UI-only preferences shared by the settings pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiPreferences {
    /// Active interface language.
    pub language: Language,
    /// Active color theme.
    pub theme_mode: ThemeMode,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            language: Language::Fr,
            theme_mode: ThemeMode::Dark,
        }
    }
}

/// Draws the large sidebar navigation items.
pub fn navigation(ui: &mut egui::Ui, language: Language, selected: &mut SettingsTab) {
    for tab in SettingsTab::ALL {
        if widgets::nav_item(ui, tab.label(language), *selected == tab).clicked() {
            *selected = tab;
        }
    }
}

/// Translated label and color for a runtime state.
pub fn state_label(language: Language, state: RuntimeState) -> (&'static str, egui::Color32) {
    match state {
        RuntimeState::Starting => (tr(language, TextKey::StateStarting), theme::WARNING),
        RuntimeState::Running => (tr(language, TextKey::StateRunning), theme::SUCCESS),
        RuntimeState::Paused => (tr(language, TextKey::StatePaused), theme::WARNING),
        RuntimeState::Stopped => (tr(language, TextKey::StateStopped), theme::DANGER),
    }
}

/// Draws the selected settings page.
pub fn page(
    ui: &mut egui::Ui,
    preferences: &mut UiPreferences,
    tab: SettingsTab,
    runtime: &RuntimeHandle,
    snapshot: &RuntimeSnapshot,
    platform: PlatformInfo,
) {
    match tab {
        SettingsTab::General => general::show(ui, preferences, runtime, snapshot),
        SettingsTab::Devices => placeholder(
            ui,
            preferences.language,
            TextKey::TabDevices,
            TextKey::DevicesSubtitle,
            TextKey::EmptyDevicesTitle,
            TextKey::EmptyDevicesBody,
        ),
        SettingsTab::Actions => placeholder(
            ui,
            preferences.language,
            TextKey::TabActions,
            TextKey::ActionsSubtitle,
            TextKey::EmptyActionsTitle,
            TextKey::EmptyActionsBody,
        ),
        SettingsTab::Macros => placeholder(
            ui,
            preferences.language,
            TextKey::TabMacros,
            TextKey::MacrosSubtitle,
            TextKey::EmptyMacrosTitle,
            TextKey::EmptyMacrosBody,
        ),
        SettingsTab::About => about::show(ui, preferences.language, platform),
    }
}

fn placeholder(
    ui: &mut egui::Ui,
    language: Language,
    title: TextKey,
    subtitle: TextKey,
    empty_title: TextKey,
    empty_body: TextKey,
) {
    widgets::page_header(ui, tr(language, title), tr(language, subtitle));
    widgets::card(ui, |ui| {
        widgets::empty_state(
            ui,
            tr(language, empty_title),
            tr(language, empty_body),
            tr(language, TextKey::ComingSoonBadge),
            theme::ACCENT,
        );
    });
}
