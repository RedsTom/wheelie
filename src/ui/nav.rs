//! Settings navigation model and the UI-only preferences shared by the pages.

use crate::core::{Language, TextKey, ThemeMode, tr};

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
