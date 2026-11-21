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

    /// Declarative descriptors in display order: id, copy key and drawer icon.
    ///
    /// Adding a section now means one row here plus one renderer in
    /// `pages::render`; the sidebar, labels and icons all read this table.
    pub const DESCRIPTORS: [TabDescriptor; 5] = [
        TabDescriptor::new(
            Self::General,
            TextKey::TabGeneral,
            gpui_icons::LucideIcon::LayoutDashboard,
        ),
        TabDescriptor::new(
            Self::Devices,
            TextKey::TabDevices,
            gpui_icons::LucideIcon::SlidersHorizontal,
        ),
        TabDescriptor::new(
            Self::Actions,
            TextKey::TabActions,
            gpui_icons::LucideIcon::Waypoints,
        ),
        TabDescriptor::new(
            Self::Macros,
            TextKey::TabMacros,
            gpui_icons::LucideIcon::ScrollText,
        ),
        TabDescriptor::new(
            Self::Settings,
            TextKey::TabSettings,
            gpui_icons::LucideIcon::Settings,
        ),
    ];

    /// Descriptor of this tab.
    pub fn descriptor(self) -> &'static TabDescriptor {
        Self::DESCRIPTORS
            .iter()
            .find(|descriptor| descriptor.tab == self)
            .expect("every settings tab has a descriptor")
    }

    /// Translated tab label.
    pub fn label(self, language: Language) -> &'static str {
        tr(language, self.descriptor().label)
    }

    /// Drawer icon of this tab.
    pub fn icon(self) -> gpui_icons::LucideIcon {
        self.descriptor().icon
    }
}

/// One row of [`SettingsTab::DESCRIPTORS`].
pub struct TabDescriptor {
    /// Stable tab id.
    pub tab: SettingsTab,
    /// Translation key of the visible label.
    pub label: TextKey,
    /// Icon shown next to the label.
    pub icon: gpui_icons::LucideIcon,
}

impl TabDescriptor {
    /// Creates a descriptor.
    pub const fn new(tab: SettingsTab, label: TextKey, icon: gpui_icons::LucideIcon) -> Self {
        Self { tab, label, icon }
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
