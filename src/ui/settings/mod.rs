//! Settings view: navigation, preferences and page dispatch.

mod about;
mod actions;
mod devices;
mod general;
mod macros;

use crate::app::{RuntimeHandle, RuntimeSnapshot, RuntimeState};
use crate::core::{CaptureMode, Config, Control, KeyChord, Language, PlatformInfo, TextKey, tr};
use crate::ui::{ThemeMode, theme, widgets};
use eframe::egui;

/// Settings sections shown in the left navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    /// Application status and options.
    #[default]
    General,
    /// Knob device and key bindings.
    Devices,
    /// Action catalog and parameters.
    Actions,
    /// Macro editor.
    Macros,
    /// Appearance, language and information about Wheelie.
    Settings,
}

impl SettingsTab {
    /// All tabs, in display order.
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Devices,
        Self::Actions,
        Self::Macros,
        Self::Settings,
    ];

    /// Tabs in the main navigation; Settings is pinned to the bottom.
    pub const MAIN: [Self; 4] = [Self::General, Self::Devices, Self::Actions, Self::Macros];

    /// Translated tab label.
    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::General => tr(language, TextKey::TabGeneral),
            Self::Devices => tr(language, TextKey::TabDevices),
            Self::Actions => tr(language, TextKey::TabActions),
            Self::Macros => tr(language, TextKey::TabMacros),
            Self::Settings => tr(language, TextKey::TabSettings),
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

/// Services and transient state pages need from the application shell.
pub struct PageContext<'a> {
    /// Runtime handle used to read and persist the configuration.
    pub runtime: &'a RuntimeHandle,
    /// Active interface language.
    pub language: Language,
    /// Static platform capabilities.
    pub platform: PlatformInfo,
    /// Brand texture used by empty states.
    pub logo: &'a egui::TextureHandle,
    /// Last chord received while the live key check is active.
    pub last_key: Option<KeyChord>,
    /// Control currently being checked, if any.
    pub testing: Option<Control>,
    /// Page request: start checking a control, or stop with `Some(None)`.
    pub test_request: Option<Option<Control>>,
    /// Set when key bindings changed and the shell must watch new keys.
    pub bindings_changed: bool,
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
    context: &mut PageContext<'_>,
    snapshot: &RuntimeSnapshot,
) {
    match tab {
        SettingsTab::General => general::show(ui, preferences, context, snapshot),
        SettingsTab::Devices => devices::show(ui, context),
        SettingsTab::Actions => actions::show(ui, context),
        SettingsTab::Macros => macros::show(ui, context),
        SettingsTab::Settings => about::show(ui, context),
    }
}

/// Index of the active profile in the configuration, when it exists.
pub fn active_profile_index(config: &Config) -> Option<usize> {
    let id = config.active_profile.as_deref()?;
    config.profiles.iter().position(|profile| profile.id == id)
}

/// Small inline warning used by pages that validate user input.
pub fn warning(ui: &mut egui::Ui, text: &str) {
    ui.colored_label(theme::WARNING, text);
}

/// Platform permission or limitation notice relevant to knob keys.
pub fn platform_notice(ui: &mut egui::Ui, language: Language, platform: PlatformInfo) {
    let key = match platform.capture {
        CaptureMode::PermissionedHotkeys => Some(TextKey::MacPermissionNotice),
        CaptureMode::X11BestEffort => Some(TextKey::LinuxCaptureNotice),
        CaptureMode::ExclusiveHotkeys => Some(TextKey::WindowsElevatedNotice),
        CaptureMode::Unsupported => None,
    };

    if let Some(key) = key {
        warning(ui, tr(language, key));
        ui.add_space(10.0);
    }
}
