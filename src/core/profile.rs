//! A profile binds the three knob controls to key chords.
//!
//! Two devices emitting the same chords are indistinguishable in v1. A profile
//! may carry a `device_id` so a future version can tell devices apart, but the
//! uniqueness rule remains conservative: two profiles whose device identifiers
//! overlap must not share a chord, whatever the control. An absent identifier
//! means "any device", so it overlaps with every other profile.

use crate::core::action::Action;
use crate::core::gesture::{Bindings, BindingsError, Control};
use crate::core::key::KeyChord;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Actions assigned to the three controls of an action set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actions {
    /// Action for left rotation.
    pub left: Action,
    /// Action for right rotation.
    pub right: Action,
    /// Action for knob press.
    pub press: Action,
}

impl Default for Actions {
    fn default() -> Self {
        Self {
            left: Action::None,
            right: Action::None,
            press: Action::None,
        }
    }
}

impl Actions {
    /// Returns the action for a control.
    pub fn for_control(&self, control: Control) -> &Action {
        match control {
            Control::Left => &self.left,
            Control::Right => &self.right,
            Control::Press => &self.press,
        }
    }

    /// Mutable access to the action of a control.
    pub fn for_control_mut(&mut self, control: Control) -> &mut Action {
        match control {
            Control::Left => &mut self.left,
            Control::Right => &mut self.right,
            Control::Press => &mut self.press,
        }
    }
}

/// One knob profile: the key chords of its three controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Stable identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether the profile may be active.
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// Optional hardware identifier, reserved for future versions.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Key chords for the three controls.
    pub bindings: Bindings,
}

fn enabled_by_default() -> bool {
    true
}

impl Profile {
    /// Creates a profile.
    pub fn new(id: impl Into<String>, name: impl Into<String>, bindings: Bindings) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled: true,
            device_id: None,
            bindings,
        }
    }

    /// True when this profile can drive the given device identifier.
    ///
    /// A profile without an identifier matches every device; otherwise only an
    /// exact match is accepted.
    pub fn matches_device(&self, device_id: Option<&str>) -> bool {
        match (&self.device_id, device_id) {
            (None, _) => true,
            (Some(expected), Some(actual)) => expected == actual,
            (Some(_), None) => false,
        }
    }

    /// Checks the profile structure.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.id.trim().is_empty() {
            return Err(ProfileError::EmptyId);
        }
        if self.name.trim().is_empty() {
            return Err(ProfileError::EmptyName);
        }
        self.bindings.validate()?;
        Ok(())
    }

    /// Maps a chord to its control, if this profile binds it.
    pub fn matches_chord(&self, chord: KeyChord) -> Option<Control> {
        self.bindings.control_for(chord)
    }
}

/// Error returned when a profile is invalid.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProfileError {
    /// The identifier is empty.
    #[error("profile id cannot be empty")]
    EmptyId,

    /// The display name is empty.
    #[error("profile name cannot be empty")]
    EmptyName,

    /// The bindings are ambiguous.
    #[error(transparent)]
    Bindings(#[from] BindingsError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::{MediaCommand, VolumeDirection};
    use crate::core::gesture::{
        DEFAULT_DOUBLE_CLICK_MS, DEFAULT_ROTATION_DEBOUNCE_MS, GestureConfig,
    };
    use crate::core::key::KeyCode;

    fn bindings() -> Bindings {
        Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        }
    }

    #[test]
    fn default_profile_is_valid() {
        let profile = Profile::new("default", "Default", bindings());
        assert_eq!(profile.validate(), Ok(()));
        assert!(profile.enabled);
    }

    #[test]
    fn profile_rejects_empty_id_and_name() {
        let mut profile = Profile::new("default", "Default", bindings());
        profile.id = " ".to_owned();
        assert_eq!(profile.validate(), Err(ProfileError::EmptyId));

        profile.id = "default".to_owned();
        profile.name = " ".to_owned();
        assert_eq!(profile.validate(), Err(ProfileError::EmptyName));
    }

    #[test]
    fn profile_rejects_duplicate_bindings() {
        let bindings = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F13),
            press: KeyChord::new(KeyCode::F15),
        };
        let profile = Profile::new("default", "Default", bindings);
        assert_eq!(
            profile.validate(),
            Err(ProfileError::Bindings(BindingsError::LeftEqualsRight))
        );
    }

    #[test]
    fn profile_maps_chords_to_controls() {
        let profile = Profile::new("default", "Default", bindings());
        assert_eq!(
            profile.matches_chord(KeyChord::new(KeyCode::F13)),
            Some(Control::Left)
        );
        assert_eq!(profile.matches_chord(KeyChord::new(KeyCode::F20)), None);
    }

    #[test]
    fn actions_return_the_control_action() {
        let mut actions = Actions {
            left: Action::Media {
                command: MediaCommand::Next,
            },
            ..Default::default()
        };
        assert_eq!(
            actions.for_control(Control::Left),
            &Action::Media {
                command: MediaCommand::Next
            }
        );
        assert_eq!(actions.for_control(Control::Press), &Action::None);

        *actions.for_control_mut(Control::Press) = Action::Volume {
            direction: VolumeDirection::Down,
            steps: 1,
        };
        assert!(matches!(
            actions.for_control(Control::Press),
            Action::Volume { .. }
        ));
    }

    #[test]
    fn gesture_config_uses_the_shared_defaults() {
        // Guards the shared defaults used by the runtime.
        assert_eq!(
            GestureConfig::default().double_click_ms,
            DEFAULT_DOUBLE_CLICK_MS
        );
        assert_eq!(
            GestureConfig::default().rotation_debounce_ms,
            DEFAULT_ROTATION_DEBOUNCE_MS
        );
    }

    #[test]
    fn device_identifier_defaults_to_any_device() {
        let profile = Profile::new("default", "Default", bindings());
        assert_eq!(profile.device_id, None);
        assert!(profile.matches_device(None));
        assert!(profile.matches_device(Some("keyboard-1")));

        let serial = Profile {
            device_id: Some("keyboard-1".to_owned()),
            ..Profile::new("serial", "Serial", bindings())
        };
        assert!(serial.matches_device(Some("keyboard-1")));
        assert!(!serial.matches_device(Some("keyboard-2")));
        assert!(!serial.matches_device(None));
    }

    #[test]
    fn device_identifier_is_absent_from_older_documents() {
        #[derive(serde::Deserialize)]
        struct LegacyProfile {
            #[serde(default)]
            device_id: Option<String>,
        }

        let row: LegacyProfile = toml::from_str("name = \"Default\"").expect("deserialize");
        assert_eq!(row.device_id, None);
    }
}
