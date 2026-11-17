//! A profile binds three knob controls to keys and actions.

use crate::core::action::{Action, ActionError};
use crate::core::gesture::{Bindings, BindingsError, Control};
use crate::core::key::KeyChord;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Actions assigned to the three controls of a profile.
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
}

/// One knob profile: its keys and the actions they trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Stable identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether the profile may be active.
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// Key chords for the three controls.
    pub bindings: Bindings,
    /// Actions for the three controls.
    #[serde(default)]
    pub actions: Actions,
}

fn enabled_by_default() -> bool {
    true
}

impl Profile {
    /// Creates a profile with no actions.
    pub fn new(id: impl Into<String>, name: impl Into<String>, bindings: Bindings) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled: true,
            bindings,
            actions: Actions::default(),
        }
    }

    /// Checks the profile against the available macro names.
    pub fn validate(&self, macro_names: &HashSet<String>) -> Result<(), ProfileError> {
        if self.id.trim().is_empty() {
            return Err(ProfileError::EmptyId);
        }
        if self.name.trim().is_empty() {
            return Err(ProfileError::EmptyName);
        }
        self.bindings.validate()?;

        for (control, action) in [
            (Control::Left, &self.actions.left),
            (Control::Right, &self.actions.right),
            (Control::Press, &self.actions.press),
        ] {
            action
                .validate(macro_names)
                .map_err(|source| ProfileError::InvalidAction {
                    control: control_name(control),
                    source,
                })?;
        }

        Ok(())
    }

    /// Maps a chord to its control, if this profile binds it.
    pub fn matches_chord(&self, chord: KeyChord) -> Option<Control> {
        self.bindings.control_for(chord)
    }
}

const fn control_name(control: Control) -> &'static str {
    match control {
        Control::Left => "left",
        Control::Right => "right",
        Control::Press => "press",
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

    /// One of the assigned actions is invalid.
    #[error("invalid action for {control}: {source}")]
    InvalidAction {
        /// Control name.
        control: &'static str,
        /// Underlying action error.
        #[source]
        source: ActionError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::{MediaCommand, MouseButton, VolumeDirection};
    use crate::core::gesture::GestureConfig;
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
        assert_eq!(profile.validate(&HashSet::new()), Ok(()));
        assert!(profile.enabled);
    }

    #[test]
    fn profile_rejects_empty_id_and_name() {
        let mut profile = Profile::new("default", "Default", bindings());
        profile.id = " ".to_owned();
        assert_eq!(
            profile.validate(&HashSet::new()),
            Err(ProfileError::EmptyId)
        );

        profile.id = "default".to_owned();
        profile.name = " ".to_owned();
        assert_eq!(
            profile.validate(&HashSet::new()),
            Err(ProfileError::EmptyName)
        );
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
            profile.validate(&HashSet::new()),
            Err(ProfileError::Bindings(BindingsError::LeftEqualsRight))
        );
    }

    #[test]
    fn profile_checks_action_references() {
        let mut profile = Profile::new("default", "Default", bindings());
        profile.actions.left = Action::Macro {
            name: "missing".to_owned(),
        };

        let error = profile.validate(&HashSet::new()).expect_err("must fail");
        assert!(matches!(
            error,
            ProfileError::InvalidAction {
                control: "left",
                ..
            }
        ));
    }

    #[test]
    fn profile_accepts_known_macro_and_valid_actions() {
        let mut profile = Profile::new("default", "Default", bindings());
        profile.actions.press = Action::Macro {
            name: "copy".to_owned(),
        };
        profile.actions.right = Action::Volume {
            direction: VolumeDirection::Up,
            steps: 2,
        };
        profile.actions.left = Action::MouseClick {
            button: MouseButton::Left,
        };

        let mut macros = HashSet::new();
        macros.insert("copy".to_owned());
        assert_eq!(profile.validate(&macros), Ok(()));
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
        let actions = Actions {
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
    }

    #[test]
    fn gesture_config_is_usable_with_profiles() {
        // Guards the shared defaults used by the runtime.
        assert_eq!(GestureConfig::default().double_click_ms, 350);
    }
}
