//! Supported languages and the translation table.
//!
//! Only user-facing prose lives here. Never expose milestone names, module
//! names, architecture terms or other developer vocabulary in the UI.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextKey {
    AppTagline,
    Workspace,
    TabGeneral,
    TabDevices,
    TabActions,
    TabMacros,
    TabAbout,
    GeneralSubtitle,
    DevicesSubtitle,
    ActionsSubtitle,
    MacrosSubtitle,
    StateStarting,
    StateRunning,
    StatePaused,
    StateStopped,
    PauseAction,
    ResumeAction,
    RuntimeReadyTitle,
    RuntimeReadyBody,
    RuntimePausedTitle,
    RuntimePausedBody,
    RuntimeStoppedTitle,
    RuntimeStoppedBody,
    PreferencesTitle,
    PreferencesBody,
    LanguageDescription,
    ThemeDescription,
    GettingStartedTitle,
    GettingStartedSubtitle,
    StepConfigureTitle,
    StepConfigureBody,
    StepAssignTitle,
    StepAssignBody,
    StepOpenTitle,
    StepOpenBody,
    EmptyDevicesTitle,
    EmptyDevicesBody,
    EmptyActionsTitle,
    EmptyActionsBody,
    EmptyMacrosTitle,
    EmptyMacrosBody,
    ComingSoonBadge,
    ActiveProfileLabel,
    NoProfileLabel,
    ControlLeft,
    ControlRight,
    ControlPress,
    ActionNone,
    ActionVolumeUp,
    ActionVolumeDown,
    ActionScrollUp,
    ActionScrollDown,
    ActionScrollLeft,
    ActionScrollRight,
    ActionMacro,
    ActionShortcut,
    ActionMouseClick,
    ActionMouseLeft,
    ActionMouseRight,
    ActionMouseMiddle,
    ActionLaunch,
    ActionMediaPlayPause,
    ActionMediaNext,
    ActionMediaPrevious,
    ActionMediaStop,
    AboutTitle,
    AboutBody,
    AboutSubtitle,
    FreeSoftware,
    VersionLabel,
    LicenseLabel,
    PlatformCardTitle,
    PlatformLimitedWarning,
    ThemeLabel,
    LanguageLabel,
    DarkTheme,
    LightTheme,
    LastErrorLabel,
}

impl TextKey {
    /// Every key, used by completeness tests.
    pub const ALL: &'static [Self] = &[
        Self::AppTagline,
        Self::Workspace,
        Self::TabGeneral,
        Self::TabDevices,
        Self::TabActions,
        Self::TabMacros,
        Self::TabAbout,
        Self::GeneralSubtitle,
        Self::DevicesSubtitle,
        Self::ActionsSubtitle,
        Self::MacrosSubtitle,
        Self::StateStarting,
        Self::StateRunning,
        Self::StatePaused,
        Self::StateStopped,
        Self::PauseAction,
        Self::ResumeAction,
        Self::RuntimeReadyTitle,
        Self::RuntimeReadyBody,
        Self::RuntimePausedTitle,
        Self::RuntimePausedBody,
        Self::RuntimeStoppedTitle,
        Self::RuntimeStoppedBody,
        Self::PreferencesTitle,
        Self::PreferencesBody,
        Self::LanguageDescription,
        Self::ThemeDescription,
        Self::GettingStartedTitle,
        Self::GettingStartedSubtitle,
        Self::StepConfigureTitle,
        Self::StepConfigureBody,
        Self::StepAssignTitle,
        Self::StepAssignBody,
        Self::StepOpenTitle,
        Self::StepOpenBody,
        Self::EmptyDevicesTitle,
        Self::EmptyDevicesBody,
        Self::EmptyActionsTitle,
        Self::EmptyActionsBody,
        Self::EmptyMacrosTitle,
        Self::EmptyMacrosBody,
        Self::ComingSoonBadge,
        Self::ActiveProfileLabel,
        Self::NoProfileLabel,
        Self::ControlLeft,
        Self::ControlRight,
        Self::ControlPress,
        Self::ActionNone,
        Self::ActionVolumeUp,
        Self::ActionVolumeDown,
        Self::ActionScrollUp,
        Self::ActionScrollDown,
        Self::ActionScrollLeft,
        Self::ActionScrollRight,
        Self::ActionMacro,
        Self::ActionShortcut,
        Self::ActionMouseClick,
        Self::ActionMouseLeft,
        Self::ActionMouseRight,
        Self::ActionMouseMiddle,
        Self::ActionLaunch,
        Self::ActionMediaPlayPause,
        Self::ActionMediaNext,
        Self::ActionMediaPrevious,
        Self::ActionMediaStop,
        Self::AboutTitle,
        Self::AboutBody,
        Self::AboutSubtitle,
        Self::FreeSoftware,
        Self::VersionLabel,
        Self::LicenseLabel,
        Self::PlatformCardTitle,
        Self::PlatformLimitedWarning,
        Self::ThemeLabel,
        Self::LanguageLabel,
        Self::DarkTheme,
        Self::LightTheme,
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
        key: TextKey::Workspace,
        fr: "Paramètres",
        en: "Settings",
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
        key: TextKey::GeneralSubtitle,
        fr: "État de Wheelie, apparence et langue.",
        en: "Wheelie status, appearance and language.",
    },
    Translation {
        key: TextKey::DevicesSubtitle,
        fr: "Associez les touches de votre knob.",
        en: "Map the keys of your keyboard knob.",
    },
    Translation {
        key: TextKey::ActionsSubtitle,
        fr: "Choisissez ce que fait chaque geste du knob.",
        en: "Choose what each knob gesture does.",
    },
    Translation {
        key: TextKey::MacrosSubtitle,
        fr: "Séquences réutilisables de touches, texte et délais.",
        en: "Reusable sequences of keys, text and delays.",
    },
    Translation {
        key: TextKey::StateStarting,
        fr: "Démarrage",
        en: "Starting",
    },
    Translation {
        key: TextKey::StateRunning,
        fr: "Prêt",
        en: "Ready",
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
        key: TextKey::RuntimeReadyTitle,
        fr: "Wheelie est prêt",
        en: "Wheelie is ready",
    },
    Translation {
        key: TextKey::RuntimeReadyBody,
        fr: "Configurez un knob pour commencer à utiliser la roue.",
        en: "Configure a knob to start using the wheel.",
    },
    Translation {
        key: TextKey::RuntimePausedTitle,
        fr: "Wheelie est en pause",
        en: "Wheelie is paused",
    },
    Translation {
        key: TextKey::RuntimePausedBody,
        fr: "Wheelie ne réagit pas au knob pendant la pause.",
        en: "Wheelie does not respond to the knob while paused.",
    },
    Translation {
        key: TextKey::RuntimeStoppedTitle,
        fr: "Wheelie est arrêté",
        en: "Wheelie is stopped",
    },
    Translation {
        key: TextKey::RuntimeStoppedBody,
        fr: "Redémarrez Wheelie pour qu'il réponde à nouveau.",
        en: "Restart Wheelie so it can respond again.",
    },
    Translation {
        key: TextKey::PreferencesTitle,
        fr: "Préférences",
        en: "Preferences",
    },
    Translation {
        key: TextKey::PreferencesBody,
        fr: "Choisissez l'apparence et la langue de Wheelie.",
        en: "Choose how Wheelie looks and speaks.",
    },
    Translation {
        key: TextKey::LanguageDescription,
        fr: "Langue utilisée dans toute l'interface.",
        en: "Language used across the interface.",
    },
    Translation {
        key: TextKey::ThemeDescription,
        fr: "Apparence sombre ou claire.",
        en: "Dark or light appearance.",
    },
    Translation {
        key: TextKey::GettingStartedTitle,
        fr: "Pour démarrer",
        en: "Getting started",
    },
    Translation {
        key: TextKey::GettingStartedSubtitle,
        fr: "Trois étapes pour utiliser votre knob.",
        en: "Three steps to use your knob.",
    },
    Translation {
        key: TextKey::StepConfigureTitle,
        fr: "Configurez votre knob",
        en: "Configure your knob",
    },
    Translation {
        key: TextKey::StepConfigureBody,
        fr: "Choisissez les touches que votre logiciel clavier associe à la gauche, la droite et la pression.",
        en: "Choose the keys your keyboard software assigns to left, right and press.",
    },
    Translation {
        key: TextKey::StepAssignTitle,
        fr: "Assignez vos actions",
        en: "Assign your actions",
    },
    Translation {
        key: TextKey::StepAssignBody,
        fr: "Volume, défilement, macro, raccourci, souris ou lancement d'application.",
        en: "Volume, scroll, macro, shortcut, mouse or app launch.",
    },
    Translation {
        key: TextKey::StepOpenTitle,
        fr: "Ouvrez la roue",
        en: "Open the wheel",
    },
    Translation {
        key: TextKey::StepOpenBody,
        fr: "Double-cliquez sur le knob pour afficher la roue et choisir une action.",
        en: "Double-click the knob to open the wheel and choose an action.",
    },
    Translation {
        key: TextKey::EmptyDevicesTitle,
        fr: "Aucun knob configuré",
        en: "No knob configured",
    },
    Translation {
        key: TextKey::EmptyDevicesBody,
        fr: "Choisissez les touches de votre knob pour la gauche, la droite et la pression. Cette section arrive bientôt.",
        en: "Choose the keys of your knob for left, right and press. This section is coming soon.",
    },
    Translation {
        key: TextKey::EmptyActionsTitle,
        fr: "Aucune action",
        en: "No actions yet",
    },
    Translation {
        key: TextKey::EmptyActionsBody,
        fr: "Choisissez une action de volume, de défilement, de macro, de raccourci, de souris ou de lancement pour chaque geste.",
        en: "Choose a volume, scroll, macro, shortcut, mouse or launch action for each gesture.",
    },
    Translation {
        key: TextKey::EmptyMacrosTitle,
        fr: "Aucune macro",
        en: "No macros yet",
    },
    Translation {
        key: TextKey::EmptyMacrosBody,
        fr: "Créez des séquences de touches, de texte et de délais à rejouer depuis la roue. Cette section arrive bientôt.",
        en: "Create sequences of keys, text and delays to replay from the wheel. This section is coming soon.",
    },
    Translation {
        key: TextKey::ComingSoonBadge,
        fr: "Bientôt",
        en: "Coming soon",
    },
    Translation {
        key: TextKey::ActiveProfileLabel,
        fr: "Profil actif",
        en: "Active profile",
    },
    Translation {
        key: TextKey::NoProfileLabel,
        fr: "Aucun profil sélectionné",
        en: "No profile selected",
    },
    Translation {
        key: TextKey::ControlLeft,
        fr: "Rotation gauche",
        en: "Rotate left",
    },
    Translation {
        key: TextKey::ControlRight,
        fr: "Rotation droite",
        en: "Rotate right",
    },
    Translation {
        key: TextKey::ControlPress,
        fr: "Pression",
        en: "Press",
    },
    Translation {
        key: TextKey::ActionNone,
        fr: "Aucune action",
        en: "No action",
    },
    Translation {
        key: TextKey::ActionVolumeUp,
        fr: "Volume +",
        en: "Volume up",
    },
    Translation {
        key: TextKey::ActionVolumeDown,
        fr: "Volume −",
        en: "Volume down",
    },
    Translation {
        key: TextKey::ActionScrollUp,
        fr: "Défiler vers le haut",
        en: "Scroll up",
    },
    Translation {
        key: TextKey::ActionScrollDown,
        fr: "Défiler vers le bas",
        en: "Scroll down",
    },
    Translation {
        key: TextKey::ActionScrollLeft,
        fr: "Défiler vers la gauche",
        en: "Scroll left",
    },
    Translation {
        key: TextKey::ActionScrollRight,
        fr: "Défiler vers la droite",
        en: "Scroll right",
    },
    Translation {
        key: TextKey::ActionMacro,
        fr: "Macro",
        en: "Macro",
    },
    Translation {
        key: TextKey::ActionShortcut,
        fr: "Raccourci",
        en: "Shortcut",
    },
    Translation {
        key: TextKey::ActionMouseClick,
        fr: "Clic souris",
        en: "Mouse click",
    },
    Translation {
        key: TextKey::ActionMouseLeft,
        fr: "Clic gauche",
        en: "Left click",
    },
    Translation {
        key: TextKey::ActionMouseRight,
        fr: "Clic droit",
        en: "Right click",
    },
    Translation {
        key: TextKey::ActionMouseMiddle,
        fr: "Clic molette",
        en: "Middle click",
    },
    Translation {
        key: TextKey::ActionLaunch,
        fr: "Lancer une application",
        en: "Launch app",
    },
    Translation {
        key: TextKey::ActionMediaPlayPause,
        fr: "Lecture/Pause",
        en: "Play/Pause",
    },
    Translation {
        key: TextKey::ActionMediaNext,
        fr: "Piste suivante",
        en: "Next track",
    },
    Translation {
        key: TextKey::ActionMediaPrevious,
        fr: "Piste précédente",
        en: "Previous track",
    },
    Translation {
        key: TextKey::ActionMediaStop,
        fr: "Arrêter",
        en: "Stop",
    },
    Translation {
        key: TextKey::AboutTitle,
        fr: "À propos de Wheelie",
        en: "About Wheelie",
    },
    Translation {
        key: TextKey::AboutBody,
        fr: "Wheelie reproduit l'expérience de la roue Windows avec ses propres actions. Ce n'est pas un émulateur de la Windows Wheel.",
        en: "Wheelie recreates the Windows Wheel experience with its own actions. It is not a Windows Wheel emulator.",
    },
    Translation {
        key: TextKey::AboutSubtitle,
        fr: "Version, licence et informations système.",
        en: "Version, license and system information.",
    },
    Translation {
        key: TextKey::FreeSoftware,
        fr: "Logiciel libre sous GPL-3.0.",
        en: "Free software under GPL-3.0.",
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
        key: TextKey::PlatformCardTitle,
        fr: "Système",
        en: "System",
    },
    Translation {
        key: TextKey::PlatformLimitedWarning,
        fr: "Certaines touches peuvent ne pas fonctionner selon les autorisations système.",
        en: "Some keys may not work depending on system permissions.",
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
        key: TextKey::LastErrorLabel,
        fr: "Dernière erreur",
        en: "Last error",
    },
];

/// Returns the translated string for `key`.
///
/// Falls back to a visible placeholder so a missing row is caught by tests
/// instead of panicking in production.
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

    #[test]
    fn translations_do_not_expose_developer_vocabulary() {
        let forbidden = [
            "M0",
            "M1",
            "M2",
            "M3",
            "M4",
            "M5",
            "M6",
            "runtime",
            "hotkey",
            "hotkeys",
            "core",
            "platform",
            "architecture",
            "workspace",
            "milestone",
            "jalon",
            "domaine",
            "tests",
            "simulation",
            "roadmap",
            "feuille de route",
        ];

        for row in TRANSLATIONS {
            for term in forbidden {
                assert!(
                    !row.fr
                        .to_ascii_lowercase()
                        .contains(&term.to_ascii_lowercase()),
                    "developer term {term:?} leaked into French UI string: {}",
                    row.fr
                );
                assert!(
                    !row.en
                        .to_ascii_lowercase()
                        .contains(&term.to_ascii_lowercase()),
                    "developer term {term:?} leaked into English UI string: {}",
                    row.en
                );
            }
        }
    }
}
