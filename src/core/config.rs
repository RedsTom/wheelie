//! Versioned application configuration.

use crate::core::action_set::{
    ActionSet, SCROLL_HORIZONTAL_SET_ID, default_sets, horizontal_scroll_set,
};
use crate::core::gesture::{GestureConfig, GestureConfigError};
use crate::core::i18n::Language;
use crate::core::macro_engine::{MacroDefinition, MacroError};
use crate::core::profile::{Profile, ProfileError};
use crate::core::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Current configuration schema version.
///
/// Version 2 adds the action sets selectable from the wheel; version 3 adds the
/// default horizontal scroll set.
pub const CONFIG_VERSION: u32 = 3;

/// First schema version, kept so old documents can be upgraded in place.
const LEGACY_VERSION_1: u32 = 1;
/// Second schema version.
const LEGACY_VERSION_2: u32 = 2;

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
    /// Identifier of the active action set, when one is selected.
    #[serde(default)]
    pub active_set: Option<String>,
    /// Action sets selectable from the wheel.
    #[serde(default)]
    pub action_sets: Vec<ActionSet>,
    /// Available macros.
    #[serde(default)]
    pub macros: Vec<MacroDefinition>,
}

impl Default for Config {
    fn default() -> Self {
        Self::default_with(true)
    }
}

impl Config {
    /// Default document for a fresh installation.
    ///
    /// `brightness_supported` controls whether the default brightness set is
    /// offered: platforms whose input backend cannot change it must not show it.
    pub fn default_with(brightness_supported: bool) -> Self {
        use crate::core::gesture::Bindings;
        use crate::core::key::KeyCode;

        let bindings = Bindings {
            left: KeyCode::F13.into(),
            right: KeyCode::F14.into(),
            press: KeyCode::F15.into(),
        };

        let language = Language::Fr;
        let sets = default_sets(language, brightness_supported);
        let active_set = sets.first().map(|set| set.id.clone());

        Self {
            version: CONFIG_VERSION,
            language,
            general: GeneralConfig::default(),
            active_profile: Some("default".to_owned()),
            profiles: vec![Profile::new("default", "Default", bindings)],
            active_set,
            action_sets: sets,
            macros: Vec::new(),
        }
    }

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

    /// The active action set, if it exists.
    pub fn active_set(&self) -> Option<&ActionSet> {
        let id = self.active_set.as_deref()?;
        self.action_sets.iter().find(|set| set.id == id)
    }

    /// The active set, falling back to the first available one.
    pub fn active_set_or_first(&self) -> Option<&ActionSet> {
        self.active_set().or_else(|| self.action_sets.first())
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
            profile
                .validate()
                .map_err(|source| ConfigError::InvalidProfile {
                    id: profile.id.clone(),
                    source,
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

        let known_macros = self.macro_names();
        let mut set_ids = HashSet::new();
        let mut set_names = HashSet::new();
        for set in &self.action_sets {
            if set.id.trim().is_empty() {
                return Err(ConfigError::EmptySetId);
            }
            if !set_ids.insert(set.id.clone()) {
                return Err(ConfigError::DuplicateSetId(set.id.clone()));
            }
            if set.name.trim().is_empty() {
                return Err(ConfigError::EmptySetName(set.id.clone()));
            }
            if !set_names.insert(set.name.to_lowercase()) {
                return Err(ConfigError::DuplicateSetName(set.name.clone()));
            }
            for (control, action) in [
                ("left", &set.actions.left),
                ("right", &set.actions.right),
                ("press", &set.actions.press),
            ] {
                action
                    .validate(&known_macros)
                    .map_err(|source| ConfigError::InvalidSetAction {
                        id: set.id.clone(),
                        control,
                        source,
                    })?;
            }
        }

        if let Some(active) = self.active_profile.as_deref()
            && !profile_ids.contains(active)
        {
            return Err(ConfigError::MissingActiveProfile(active.to_owned()));
        }

        if let Some(active) = self.active_set.as_deref()
            && !set_ids.contains(active)
        {
            return Err(ConfigError::MissingActiveSet(active.to_owned()));
        }

        Ok(())
    }

    /// Parses and validates a TOML document, upgrading older versions.
    pub fn from_toml_str(document: &str) -> Result<Self, ConfigError> {
        let mut config: Self = toml::from_str(document)?;
        config.upgrade();
        config.validate()?;
        Ok(config)
    }

    /// Validates and serializes the document to pretty TOML.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }

    fn upgrade(&mut self) {
        if self.version == LEGACY_VERSION_1 {
            if self.action_sets.is_empty() {
                self.action_sets = default_sets(self.language, true);
            }
            if self.active_set.is_none() {
                self.active_set = self.action_sets.first().map(|set| set.id.clone());
            }
            self.version = LEGACY_VERSION_2;
        }

        if self.version == LEGACY_VERSION_2 {
            if !self.action_sets.is_empty()
                && !self
                    .action_sets
                    .iter()
                    .any(|set| set.id == SCROLL_HORIZONTAL_SET_ID)
            {
                self.action_sets.push(horizontal_scroll_set(self.language));
            }
            self.version = CONFIG_VERSION;
        }
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

    /// An action set is invalid.
    #[error("invalid action set {id}: {source}")]
    InvalidSetAction {
        /// Set identifier.
        id: String,
        /// Control name.
        control: &'static str,
        /// Underlying action error.
        #[source]
        source: crate::core::action::ActionError,
    },

    /// An action set has no identifier.
    #[error("action set id cannot be empty")]
    EmptySetId,

    /// Two action sets share an identifier.
    #[error("duplicate action set id: {0}")]
    DuplicateSetId(String),

    /// An action set has no name.
    #[error("action set {0} has no name")]
    EmptySetName(String),

    /// Two action sets share a name.
    #[error("duplicate action set name: {0}")]
    DuplicateSetName(String),

    /// The active profile does not exist.
    #[error("active profile not found: {0}")]
    MissingActiveProfile(String),

    /// The active action set does not exist.
    #[error("active action set not found: {0}")]
    MissingActiveSet(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::{Action, VolumeDirection};
    use crate::core::action_set::{BRIGHTNESS_SET_ID, VOLUME_SET_ID};
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
        assert!(config.active_set().is_some());
        assert_eq!(config.action_sets.len(), 4);
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
    fn version_one_documents_are_upgraded() {
        let config = Config {
            version: 1,
            action_sets: Vec::new(),
            active_set: None,
            ..Default::default()
        };
        let document = toml::to_string_pretty(&config).expect("serialize");

        let parsed = Config::from_toml_str(&document).expect("upgrade");
        assert_eq!(parsed.version, CONFIG_VERSION);
        assert_eq!(parsed.action_sets.len(), 4);
        assert_eq!(parsed.active_set.as_deref(), Some(VOLUME_SET_ID));
        assert!(
            parsed
                .action_sets
                .iter()
                .any(|set| set.id == SCROLL_HORIZONTAL_SET_ID)
        );
    }

    #[test]
    fn version_two_documents_gain_the_horizontal_scroll_set() {
        let config = Config {
            version: 2,
            action_sets: default_sets(Language::Fr, true)
                .into_iter()
                .filter(|set| set.id != SCROLL_HORIZONTAL_SET_ID)
                .collect(),
            ..Default::default()
        };
        let document = toml::to_string_pretty(&config).expect("serialize");

        let parsed = Config::from_toml_str(&document).expect("upgrade");
        assert_eq!(parsed.version, CONFIG_VERSION);
        assert!(
            parsed
                .action_sets
                .iter()
                .any(|set| set.id == SCROLL_HORIZONTAL_SET_ID)
        );
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
    fn config_rejects_unknown_active_set() {
        let config = Config {
            active_set: Some("missing".to_owned()),
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(ConfigError::MissingActiveSet(id)) if id == "missing"
        ));
    }

    #[test]
    fn config_rejects_unknown_macro_reference_in_a_set() {
        let mut config = Config::default();
        config.action_sets[0].actions.press = Action::Macro {
            name: "missing".to_owned(),
        };
        let error = config.validate().expect_err("must fail");
        assert!(matches!(
            error,
            ConfigError::InvalidSetAction { id, control: "press", .. } if id == VOLUME_SET_ID
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
    fn config_rejects_duplicate_or_nameless_sets() {
        let mut config = Config::default();
        let mut duplicate = config.action_sets[0].clone();
        duplicate.name = "Other".to_owned();
        config.action_sets.push(duplicate);
        assert!(matches!(
            config.validate(),
            Err(ConfigError::DuplicateSetId(id)) if id == VOLUME_SET_ID
        ));

        let mut config = Config::default();
        config.action_sets[1].name = config.action_sets[0].name.clone();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::DuplicateSetName(_))
        ));

        let mut config = Config::default();
        config.action_sets[0].name = " ".to_owned();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::EmptySetName(id)) if id == VOLUME_SET_ID
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

    #[test]
    fn default_sets_provide_working_actions() {
        let config = Config::default();
        let volume = &config.action_sets[0];
        assert_eq!(volume.id, VOLUME_SET_ID);
        assert!(matches!(
            volume.actions.right,
            Action::Volume {
                direction: VolumeDirection::Up,
                ..
            }
        ));

        let brightness = &config.action_sets[1];
        assert_eq!(brightness.id, BRIGHTNESS_SET_ID);
        assert!(matches!(brightness.actions.left, Action::Brightness { .. }));
    }
}
