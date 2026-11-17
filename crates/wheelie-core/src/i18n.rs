//! Supported languages and the translation table.

use serde::{Deserialize, Serialize};

/// Active user-interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// French.
    #[default]
    Fr,
    /// English.
    En,
}

impl Language {
    /// All supported languages, in display order.
    pub const ALL: [Self; 2] = [Self::Fr, Self::En];

    /// Short ISO code used in configuration files.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
        }
    }

    /// Native language name.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::Fr => "Français",
            Self::En => "English",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::str::FromStr for Language {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "fr" | "fr-fr" | "french" | "francais" | "français" => Ok(Self::Fr),
            "en" | "en-us" | "en-gb" | "english" => Ok(Self::En),
            _ => Err(()),
        }
    }
}

/// Stable keys for every translated string.
///
/// Code must never contain user-facing prose directly: add a key here and a
/// French/English row in [`TRANSLATIONS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextKey {
    AppTagline,
    Navigation,
    TabGeneral,
    TabDevices,
    TabActions,
    TabMacros,
    TabAbout,
    Status,
    StateStarting,
    StateRunning,
    StatePaused,
    StateStopped,
    CaptureActive,
    CaptureInactive,
    RuntimeCardTitle,
    PauseAction,
    ResumeAction,
    MilestoneCardTitle,
    Milestone0Title,
    Milestone0Body,
    NextSteps,
    Milestone1,
    Milestone2,
    Milestone3,
    PlatformCardTitle,
    PlatformLimitedWarning,
    AboutTitle,
    AboutBody,
    VersionLabel,
    LicenseLabel,
    RepositoryLabel,
    ArchitectureLabel,
    ArchitectureBody,
    ThemeLabel,
    LanguageLabel,
    DarkTheme,
    LightTheme,
    PlaceholderDevices,
    PlaceholderActions,
    PlaceholderMacros,
    PlannedInMilestone,
    LastErrorLabel,
}

impl TextKey {
    /// Every key, used by completeness tests.
    pub const ALL: &'static [Self] = &[
        Self::AppTagline,
        Self::Navigation,
        Self::TabGeneral,
        Self::TabDevices,
        Self::TabActions,
        Self::TabMacros,
        Self::TabAbout,
        Self::Status,
        Self::StateStarting,
        Self::StateRunning,
        Self::StatePaused,
        Self::StateStopped,
        Self::CaptureActive,
        Self::CaptureInactive,
        Self::RuntimeCardTitle,
        Self::PauseAction,
        Self::ResumeAction,
        Self::MilestoneCardTitle,
        Self::Milestone0Title,
        Self::Milestone0Body,
        Self::NextSteps,
        Self::Milestone1,
        Self::Milestone2,
        Self::Milestone3,
        Self::PlatformCardTitle,
        Self::PlatformLimitedWarning,
        Self::AboutTitle,
        Self::AboutBody,
        Self::VersionLabel,
        Self::LicenseLabel,
        Self::RepositoryLabel,
        Self::ArchitectureLabel,
        Self::ArchitectureBody,
        Self::ThemeLabel,
        Self::LanguageLabel,
        Self::DarkTheme,
        Self::LightTheme,
        Self::PlaceholderDevices,
        Self::PlaceholderActions,
        Self::PlaceholderMacros,
        Self::PlannedInMilestone,
        Self::LastErrorLabel,
    ];
}

/// One translation row.
pub struct Translation {
    key: TextKey,
    fr: &'static str,
    en: &'static str,
}

/// Translation table. Only this table contains French user-facing prose.
pub const TRANSLATIONS: &[Translation] = &[
    Translation {
        key: TextKey::AppTagline,
        fr: "La roue au bout du knob",
        en: "The wheel at your fingertips",
    },
    Translation {
        key: TextKey::Navigation,
        fr: "Navigation",
        en: "Navigation",
    },
    Translation {
        key: TextKey::TabGeneral,
        fr: "Général",
        en: "General",
    },
    Translation {
        key: TextKey::TabDevices,
        fr: "Appareils",
        en: "Devices",
    },
    Translation {
        key: TextKey::TabActions,
        fr: "Actions",
        en: "Actions",
    },
    Translation {
        key: TextKey::TabMacros,
        fr: "Macros",
        en: "Macros",
    },
    Translation {
        key: TextKey::TabAbout,
        fr: "À propos",
        en: "About",
    },
    Translation {
        key: TextKey::Status,
        fr: "État",
        en: "Status",
    },
    Translation {
        key: TextKey::StateStarting,
        fr: "Démarrage",
        en: "Starting",
    },
    Translation {
        key: TextKey::StateRunning,
        fr: "Actif",
        en: "Running",
    },
    Translation {
        key: TextKey::StatePaused,
        fr: "En pause",
        en: "Paused",
    },
    Translation {
        key: TextKey::StateStopped,
        fr: "Arrêté",
        en: "Stopped",
    },
    Translation {
        key: TextKey::CaptureActive,
        fr: "Capture active",
        en: "Capture active",
    },
    Translation {
        key: TextKey::CaptureInactive,
        fr: "Capture inactive",
        en: "Capture inactive",
    },
    Translation {
        key: TextKey::RuntimeCardTitle,
        fr: "Runtime",
        en: "Runtime",
    },
    Translation {
        key: TextKey::PauseAction,
        fr: "Mettre en pause",
        en: "Pause",
    },
    Translation {
        key: TextKey::ResumeAction,
        fr: "Reprendre",
        en: "Resume",
    },
    Translation {
        key: TextKey::MilestoneCardTitle,
        fr: "Jalon en cours",
        en: "Current milestone",
    },
    Translation {
        key: TextKey::Milestone0Title,
        fr: "M0 — Fondations",
        en: "M0 — Foundations",
    },
    Translation {
        key: TextKey::Milestone0Body,
        fr: "Workspace, CI, licence, documentation et squelette applicatif. Le cœur, les hotkeys et la roue arrivent aux jalons suivants.",
        en: "Workspace, CI, license, documentation and application skeleton. The core, hotkeys and wheel arrive in the next milestones.",
    },
    Translation {
        key: TextKey::NextSteps,
        fr: "Prochaines étapes",
        en: "Next steps",
    },
    Translation {
        key: TextKey::Milestone1,
        fr: "M1 · Domaine pur et tests",
        en: "M1 · Pure domain and tests",
    },
    Translation {
        key: TextKey::Milestone2,
        fr: "M2 · Hotkeys et simulation d'entrées",
        en: "M2 · Hotkeys and input simulation",
    },
    Translation {
        key: TextKey::Milestone3,
        fr: "M3 · Réglages, roue radiale et tray",
        en: "M3 · Settings, radial wheel and tray",
    },
    Translation {
        key: TextKey::PlatformCardTitle,
        fr: "Plateforme détectée",
        en: "Detected platform",
    },
    Translation {
        key: TextKey::PlatformLimitedWarning,
        fr: "La capture globale peut être limitée par le compositeur ou les permissions système.",
        en: "Global capture may be limited by the compositor or system permissions.",
    },
    Translation {
        key: TextKey::AboutTitle,
        fr: "À propos de Wheelie",
        en: "About Wheelie",
    },
    Translation {
        key: TextKey::AboutBody,
        fr: "Wheelie reproduit l'UI/UX de la roue Windows avec ses propres actions. Ce n'est pas un émulateur de la Windows Wheel.",
        en: "Wheelie recreates the Windows Wheel UI/UX with its own actions. It is not a Windows Wheel emulator.",
    },
    Translation {
        key: TextKey::VersionLabel,
        fr: "Version",
        en: "Version",
    },
    Translation {
        key: TextKey::LicenseLabel,
        fr: "Licence",
        en: "License",
    },
    Translation {
        key: TextKey::RepositoryLabel,
        fr: "Dépôt",
        en: "Repository",
    },
    Translation {
        key: TextKey::ArchitectureLabel,
        fr: "Architecture",
        en: "Architecture",
    },
    Translation {
        key: TextKey::ArchitectureBody,
        fr: "core → platform → app → ui, avec un binaire racine mince.",
        en: "core → platform → app → ui, with a thin root binary.",
    },
    Translation {
        key: TextKey::ThemeLabel,
        fr: "Thème",
        en: "Theme",
    },
    Translation {
        key: TextKey::LanguageLabel,
        fr: "Langue",
        en: "Language",
    },
    Translation {
        key: TextKey::DarkTheme,
        fr: "Sombre",
        en: "Dark",
    },
    Translation {
        key: TextKey::LightTheme,
        fr: "Clair",
        en: "Light",
    },
    Translation {
        key: TextKey::PlaceholderDevices,
        fr: "La sélection des touches et les profils par appareil arrivent en M1–M3.",
        en: "Key selection and device profiles arrive in M1–M3.",
    },
    Translation {
        key: TextKey::PlaceholderActions,
        fr: "Le catalogue d'actions et l'éditeur de raccourcis arrivent en M1–M3.",
        en: "The action catalog and shortcut editor arrive in M1–M3.",
    },
    Translation {
        key: TextKey::PlaceholderMacros,
        fr: "L'éditeur et le moteur de macros arrivent en M1–M3.",
        en: "The macro editor and engine arrive in M1–M3.",
    },
    Translation {
        key: TextKey::PlannedInMilestone,
        fr: "Prévu dans un prochain jalon.",
        en: "Planned for a later milestone.",
    },
    Translation {
        key: TextKey::LastErrorLabel,
        fr: "Dernière erreur",
        en: "Last error",
    },
];

/// Returns the translated string for `key`.
///
/// Falls back to English, then to a visible placeholder, so a missing row is
/// caught by tests instead of panicking in production.
pub fn tr(language: Language, key: TextKey) -> &'static str {
    let Some(translation) = TRANSLATIONS.iter().find(|row| row.key == key) else {
        return "missing translation";
    };
    match language {
        Language::Fr => translation.fr,
        Language::En => translation.en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn language_roundtrips_through_code() {
        for language in Language::ALL {
            assert_eq!(language.code().parse::<Language>(), Ok(language));
        }
    }

    #[test]
    fn language_accepts_common_aliases() {
        assert_eq!("fr-FR".parse::<Language>(), Ok(Language::Fr));
        assert_eq!("EN".parse::<Language>(), Ok(Language::En));
        assert_eq!("français".parse::<Language>(), Ok(Language::Fr));
    }

    #[test]
    fn unknown_language_is_rejected() {
        assert!("de".parse::<Language>().is_err());
    }

    #[test]
    fn every_key_has_a_translation() {
        let keys: HashSet<_> = TextKey::ALL.iter().copied().collect();
        assert_eq!(keys.len(), TextKey::ALL.len(), "duplicate TextKey in ALL");

        let translated: HashSet<_> = TRANSLATIONS.iter().map(|row| row.key).collect();
        assert_eq!(
            translated.len(),
            TRANSLATIONS.len(),
            "duplicate translation row"
        );

        for key in TextKey::ALL {
            assert!(translated.contains(key), "missing translation for {key:?}");
        }
    }

    #[test]
    fn translations_are_not_empty() {
        for row in TRANSLATIONS {
            assert!(
                !row.fr.trim().is_empty(),
                "empty French row for {:?}",
                row.key
            );
            assert!(
                !row.en.trim().is_empty(),
                "empty English row for {:?}",
                row.key
            );
        }
    }
}
