//! Versioned application configuration.

use crate::core::gesture::{GestureConfig, GestureConfigError};
use crate::core::i18n::Language;
use crate::core::macro_engine::{MacroDefinition, MacroError};
use crate::core::profile::{Profile, ProfileError};
use crate::core::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Current configuration schema version.
pub const CONFIG_VERSION: u32 = 1;

fn default_true() -> bool {
    true
}

fn default_double_click_ms() -> u64 {
    350
}

fn default_rotation_debounce_ms() -> u64 {
    20
}

/// General application settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Start Wheelie when the user logs in.
    #[serde(default)]
    pub start_at_login: bool,
    /// Keep the tray icon available.
    #[serde(default = "default_true")]
    pub show_tray: bool,
    /// Double-click window in milliseconds.
    #[serde(default = "default_double_click_ms")]
    pub double_click_ms: u64,
    /// Rotation debounce in milliseconds.
    #[serde(default = "default_rotation_debounce_ms")]
    pub rotation_debounce_ms: u64,
    /// Color theme.
    #[serde(default)]
    pub theme: ThemeMode,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            start_at_login: false,
            show_tray: true,
            double_click_ms: default_double_click_ms(),
            rotation_debounce_ms: default_rotation_debounce_ms(),
            theme: ThemeMode::default(),
        }
    }
}

impl GeneralConfig {
    /// Builds the runtime gesture settings from persisted values.
    pub fn gesture_config(&self) -> GestureConfig {
        GestureConfig {
            double_click_ms: self.double_click_ms,
            rotation_debounce_ms: self.rotation_debounce_ms,
        }
    }
}

/// Root configuration document stored as TOML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Schema version.
    pub version: u32,
    /// Interface language.
    #[serde(default)]
    pub language: Language,
    /// General settings.
    #[serde(default)]
    pub general: GeneralConfig,
    /// Identifier of the active profile, when one is selected.
    #[serde(default)]
    pub active_profile: Option<String>,
    /// Available knob profiles.
    #[serde(default)]
    pub profiles: Vec<Profile>,
    /// Available macros.
    #[serde(default)]
    pub macros: Vec<MacroDefinition>,
}

impl Default for Config {
    fn default() -> Self {
        use crate::core::gesture::Bindings;
        use crate::core::key::KeyCode;

        let bindings = Bindings {
            left: KeyCode::F13.into(),
            right: KeyCode::F14.into(),
            press: KeyCode::F15.into(),
        };

        Self {
            version: CONFIG_VERSION,
            language: Language::Fr,
            general: GeneralConfig::default(),
            active_profile: Some("default".to_owned()),
            profiles: vec![Profile::new("default", "Default", bindings)],
            macros: Vec::new(),
        }
    }
}

impl Config {
    /// Names of all macros, used to validate action references.
    pub fn macro_names(&self) -> HashSet<String> {
        self.macros
            .iter()
            .map(|macro_def| macro_def.name.clone())
            .collect()
    }

    /// The active profile, if it exists.
    pub fn active_profile(&self) -> Option<&Profile> {
        let id = self.active_profile.as_deref()?;
        self.profiles.iter().find(|profile| profile.id == id)
    }

    /// Validates the whole document.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion(self.version));
        }

        self.general
            .gesture_config()
            .validate()
            .map_err(ConfigError::from)?;

        let mut profile_ids = HashSet::new();
        for profile in &self.profiles {
            if !profile_ids.insert(profile.id.clone()) {
                return Err(ConfigError::DuplicateProfileId(profile.id.clone()));
            }
            profile.validate(&self.macro_names()).map_err(|source| {
                ConfigError::InvalidProfile {
                    id: profile.id.clone(),
                    source,
                }
            })?;
        }

        let mut macro_names = HashSet::new();
        for macro_def in &self.macros {
            if !macro_names.insert(macro_def.name.clone()) {
                return Err(ConfigError::DuplicateMacroName(macro_def.name.clone()));
            }
            macro_def
                .validate()
                .map_err(|source| ConfigError::InvalidMacro {
                    name: macro_def.name.clone(),
                    source,
                })?;
        }

        if let Some(active) = self.active_profile.as_deref()
            && !profile_ids.contains(active)
        {
            return Err(ConfigError::MissingActiveProfile(active.to_owned()));
        }

        Ok(())
    }

    /// Parses and validates a TOML document.
    pub fn from_toml_str(document: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(document)?;
        config.validate()?;
        Ok(config)
    }

    /// Validates and serializes the document to pretty TOML.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }
}

/// Error returned while loading or validating configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The TOML document could not be parsed.
    #[error("failed to parse configuration: {0}")]
    Parse(#[from] toml::de::Error),

    /// The document could not be serialized.
    #[error("failed to serialize configuration: {0}")]
    Serialize(#[from] toml::ser::Error),

    /// The schema version is not supported.
    #[error("unsupported configuration version: {0}")]
    UnsupportedVersion(u32),

    /// Gesture timing is out of range.
    #[error(transparent)]
    Gesture(#[from] GestureConfigError),

    /// A profile is invalid.
    #[error("invalid profile {id}: {source}")]
    InvalidProfile {
        /// Profile identifier.
        id: String,
        /// Underlying profile error.
        #[source]
        source: ProfileError,
    },

    /// Two profiles share an identifier.
    #[error("duplicate profile id: {0}")]
    DuplicateProfileId(String),

    /// A macro is invalid.
    #[error("invalid macro {name}: {source}")]
    InvalidMacro {
        /// Macro name.
        name: String,
        /// Underlying macro error.
        #[source]
        source: MacroError,
    },

    /// Two macros share a name.
    #[error("duplicate macro name: {0}")]
    DuplicateMacroName(String),

    /// The active profile does not exist.
    #[error("active profile not found: {0}")]
    MissingActiveProfile(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::Action;
    use crate::core::gesture::{Bindings, Control};
    use crate::core::key::{KeyChord, KeyCode};

    fn bindings() -> Bindings {
        Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        }
    }

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        config.validate().expect("default config must be valid");
        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.active_profile().is_some());
    }

    #[test]
    fn config_round_trips_through_toml() {
        let config = Config::default();
        let document = config.to_toml_string().expect("serialize");
        let parsed = Config::from_toml_str(&document).expect("deserialize");
        assert_eq!(parsed, config);
    }

    #[test]
    fn config_rejects_unsupported_version() {
        let config = Config {
            version: CONFIG_VERSION + 1,
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::UnsupportedVersion(version)) if version == CONFIG_VERSION + 1
        ));
    }

    #[test]
    fn config_rejects_duplicate_profile_ids() {
        let mut config = Config::default();
        config
            .profiles
            .push(Profile::new("default", "Other", bindings()));
        assert!(matches!(
            config.validate(),
            Err(ConfigError::DuplicateProfileId(id)) if id == "default"
        ));
    }

    #[test]
    fn config_rejects_unknown_active_profile() {
        let config = Config {
            active_profile: Some("missing".to_owned()),
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::MissingActiveProfile(id)) if id == "missing"
        ));
    }

    #[test]
    fn config_rejects_unknown_macro_reference() {
        let mut profiles = Config::default().profiles;
        profiles[0].actions.press = Action::Macro {
            name: "missing".to_owned(),
        };
        let config = Config {
            profiles,
            ..Default::default()
        };
        let error = config.validate().expect_err("must fail");
        assert!(matches!(
            error,
            ConfigError::InvalidProfile { id, .. } if id == "default"
        ));
    }

    #[test]
    fn config_rejects_duplicate_macro_names() {
        let mut config = Config::default();
        let macro_def = crate::core::MacroDefinition {
            name: "copy".to_owned(),
            steps: vec![crate::core::MacroStep::Delay { ms: 0 }],
        };
        config.macros.push(macro_def.clone());
        config.macros.push(macro_def);
        assert!(matches!(
            config.validate(),
            Err(ConfigError::DuplicateMacroName(name)) if name == "copy"
        ));
    }

    #[test]
    fn config_rejects_bad_gesture_timing() {
        let config = Config {
            general: GeneralConfig {
                double_click_ms: 10,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::Gesture(
                GestureConfigError::InvalidDoubleClick(10)
            ))
        ));
    }

    #[test]
    fn config_serializes_language_and_theme() {
        let config = Config {
            language: Language::En,
            general: GeneralConfig {
                theme: ThemeMode::Light,
                ..Default::default()
            },
            ..Default::default()
        };
        let document = config.to_toml_string().expect("serialize");
        let parsed = Config::from_toml_str(&document).expect("deserialize");
        assert_eq!(parsed.language, Language::En);
        assert_eq!(parsed.general.theme, ThemeMode::Light);
    }

    #[test]
    fn active_profile_lookup_maps_controls() {
        let config = Config::default();
        let profile = config.active_profile().expect("active profile");
        assert_eq!(
            profile.matches_chord(KeyChord::new(KeyCode::F13)),
            Some(Control::Left)
        );
    }
}
