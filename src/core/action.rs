//! Actions that can be assigned to a knob gesture.

use crate::core::i18n::{Language, TextKey, tr};
use crate::core::key::KeyChord;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Direction of a volume action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolumeDirection {
    /// Increase the system volume.
    Up,
    /// Decrease the system volume.
    Down,
}

/// Axis of a scroll action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollAxis {
    /// Vertical wheel movement.
    Vertical,
    /// Horizontal wheel movement.
    Horizontal,
}

/// Mouse button used by a click action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Right button.
    Right,
    /// Middle button.
    Middle,
}

impl MouseButton {
    /// Translated button label.
    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::Left => tr(language, TextKey::ActionMouseLeft),
            Self::Right => tr(language, TextKey::ActionMouseRight),
            Self::Middle => tr(language, TextKey::ActionMouseMiddle),
        }
    }
}

/// Media transport command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaCommand {
    /// Toggle playback.
    PlayPause,
    /// Skip to the next track.
    Next,
    /// Skip to the previous track.
    Previous,
    /// Stop playback.
    Stop,
}

/// An action executed when a knob gesture fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Do nothing.
    None,
    /// Change the system volume.
    Volume {
        /// Direction of the change.
        direction: VolumeDirection,
        /// Number of OS volume steps.
        steps: u8,
    },
    /// Scroll the window under the cursor.
    Scroll {
        /// Scroll axis.
        axis: ScrollAxis,
        /// Signed amount: negative is up/left, positive is down/right.
        amount: i32,
    },
    /// Run a named macro.
    Macro {
        /// Macro name from the configuration.
        name: String,
    },
    /// Send a keyboard shortcut.
    Shortcut {
        /// Shortcut chord.
        chord: KeyChord,
    },
    /// Click a mouse button.
    MouseClick {
        /// Button to click.
        button: MouseButton,
    },
    /// Send a media transport command.
    Media {
        /// Media command.
        command: MediaCommand,
    },
    /// Launch an application or file.
    Launch {
        /// Absolute path to launch.
        path: PathBuf,
    },
}

impl Action {
    /// A short human-readable description.
    pub fn describe(&self, language: Language) -> String {
        match self {
            Self::None => tr(language, TextKey::ActionNone).to_owned(),
            Self::Volume { direction, steps } => {
                let label = match direction {
                    VolumeDirection::Up => tr(language, TextKey::ActionVolumeUp),
                    VolumeDirection::Down => tr(language, TextKey::ActionVolumeDown),
                };
                with_steps(label, *steps)
            }
            Self::Scroll { axis, amount } => {
                let label = match (axis, *amount < 0) {
                    (ScrollAxis::Vertical, true) => tr(language, TextKey::ActionScrollUp),
                    (ScrollAxis::Vertical, false) => tr(language, TextKey::ActionScrollDown),
                    (ScrollAxis::Horizontal, true) => tr(language, TextKey::ActionScrollLeft),
                    (ScrollAxis::Horizontal, false) => tr(language, TextKey::ActionScrollRight),
                };
                with_amount(label, *amount)
            }
            Self::Macro { name } => format!("{} — {name}", tr(language, TextKey::ActionMacro)),
            Self::Shortcut { chord } => {
                format!("{} — {chord}", tr(language, TextKey::ActionShortcut))
            }
            Self::MouseClick { button } => button.label(language).to_owned(),
            Self::Media { command } => {
                let key = match command {
                    MediaCommand::PlayPause => TextKey::ActionMediaPlayPause,
                    MediaCommand::Next => TextKey::ActionMediaNext,
                    MediaCommand::Previous => TextKey::ActionMediaPrevious,
                    MediaCommand::Stop => TextKey::ActionMediaStop,
                };
                tr(language, key).to_owned()
            }
            Self::Launch { path } => {
                let target = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                format!("{} — {target}", tr(language, TextKey::ActionLaunch))
            }
        }
    }

    /// Common actions offered by the radial wheel picker.
    ///
    /// Macro, shortcut and launch actions are dynamic and added by the UI.
    pub fn catalog() -> Vec<Self> {
        vec![
            Self::None,
            Self::Volume {
                direction: VolumeDirection::Up,
                steps: 1,
            },
            Self::Volume {
                direction: VolumeDirection::Down,
                steps: 1,
            },
            Self::Scroll {
                axis: ScrollAxis::Vertical,
                amount: -3,
            },
            Self::Scroll {
                axis: ScrollAxis::Vertical,
                amount: 3,
            },
            Self::Scroll {
                axis: ScrollAxis::Horizontal,
                amount: -3,
            },
            Self::Scroll {
                axis: ScrollAxis::Horizontal,
                amount: 3,
            },
            Self::MouseClick {
                button: MouseButton::Left,
            },
            Self::MouseClick {
                button: MouseButton::Right,
            },
            Self::MouseClick {
                button: MouseButton::Middle,
            },
            Self::Media {
                command: MediaCommand::PlayPause,
            },
            Self::Media {
                command: MediaCommand::Next,
            },
            Self::Media {
                command: MediaCommand::Previous,
            },
            Self::Media {
                command: MediaCommand::Stop,
            },
        ]
    }

    /// Checks that the action can run with the given macro names.
    pub fn validate(&self, macro_names: &HashSet<String>) -> Result<(), ActionError> {
        match self {
            Self::Volume { steps, .. } => {
                if (1..=10).contains(steps) {
                    Ok(())
                } else {
                    Err(ActionError::InvalidVolumeSteps(*steps))
                }
            }
            Self::Scroll { amount, .. } => {
                if *amount == 0 || amount.unsigned_abs() > 100 {
                    Err(ActionError::InvalidScrollAmount(*amount))
                } else {
                    Ok(())
                }
            }
            Self::Macro { name } => {
                if name.trim().is_empty() {
                    Err(ActionError::EmptyMacroName)
                } else if !macro_names.contains(name) {
                    Err(ActionError::UnknownMacro(name.clone()))
                } else {
                    Ok(())
                }
            }
            Self::Launch { path } => {
                if path.as_os_str().is_empty() {
                    Err(ActionError::EmptyLaunchPath)
                } else {
                    Ok(())
                }
            }
            Self::None | Self::Shortcut { .. } | Self::MouseClick { .. } | Self::Media { .. } => {
                Ok(())
            }
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.describe(Language::En))
    }
}

/// Error returned when an action cannot be validated.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionError {
    /// Volume step count is outside the supported range.
    #[error("volume steps must be between 1 and 10, got {0}")]
    InvalidVolumeSteps(u8),

    /// Scroll amount is zero or too large.
    #[error("scroll amount must be non-zero and at most 100, got {0}")]
    InvalidScrollAmount(i32),

    /// The macro name is empty.
    #[error("macro name cannot be empty")]
    EmptyMacroName,

    /// The referenced macro does not exist.
    #[error("unknown macro: {0}")]
    UnknownMacro(String),

    /// The launch path is empty.
    #[error("launch path cannot be empty")]
    EmptyLaunchPath,
}

fn with_steps(label: &str, steps: u8) -> String {
    if steps == 1 {
        label.to_owned()
    } else {
        format!("{label} ({steps})")
    }
}

fn with_amount(label: &str, amount: i32) -> String {
    let magnitude = amount.unsigned_abs();
    if magnitude == 1 {
        label.to_owned()
    } else {
        format!("{label} ({magnitude})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::key::{KeyCode, Modifiers};

    fn macro_names() -> HashSet<String> {
        HashSet::from(["copy".to_owned()])
    }

    #[test]
    fn action_serde_round_trips_every_variant() {
        let actions = vec![
            Action::None,
            Action::Volume {
                direction: VolumeDirection::Up,
                steps: 2,
            },
            Action::Scroll {
                axis: ScrollAxis::Horizontal,
                amount: -5,
            },
            Action::Macro {
                name: "copy".to_owned(),
            },
            Action::Shortcut {
                chord: KeyChord::with_modifiers(
                    KeyCode::F13,
                    Modifiers {
                        control: true,
                        ..Default::default()
                    },
                ),
            },
            Action::MouseClick {
                button: MouseButton::Middle,
            },
            Action::Media {
                command: MediaCommand::PlayPause,
            },
            Action::Launch {
                path: PathBuf::from("/usr/bin/true"),
            },
        ];

        for action in actions {
            let serialized = toml::to_string(&action).expect("serialize");
            let deserialized: Action = toml::from_str(&serialized).expect("deserialize");
            assert_eq!(deserialized, action, "roundtrip failed for {serialized}");
        }
    }

    #[test]
    fn action_serde_uses_type_tag() {
        let serialized = toml::to_string(&Action::None).expect("serialize");
        assert!(
            serialized.contains("type"),
            "missing type tag: {serialized}"
        );
    }

    #[test]
    fn describe_is_translated() {
        assert_eq!(
            Action::Volume {
                direction: VolumeDirection::Up,
                steps: 1,
            }
            .describe(Language::Fr),
            "Volume +"
        );
        assert_eq!(
            Action::Volume {
                direction: VolumeDirection::Down,
                steps: 3,
            }
            .describe(Language::En),
            "Volume down (3)"
        );
        assert_eq!(
            Action::Macro {
                name: "copy".to_owned(),
            }
            .describe(Language::Fr),
            "Macro — copy"
        );
    }

    #[test]
    fn catalog_contains_common_actions() {
        let catalog = Action::catalog();
        assert!(catalog.len() >= 10);
        assert!(catalog.contains(&Action::None));
        assert!(catalog.contains(&Action::Media {
            command: MediaCommand::PlayPause
        }));
    }

    #[test]
    fn validation_rejects_bad_volume_and_scroll() {
        let bad_volume = Action::Volume {
            direction: VolumeDirection::Up,
            steps: 0,
        };
        assert_eq!(
            bad_volume.validate(&macro_names()),
            Err(ActionError::InvalidVolumeSteps(0))
        );

        let bad_scroll = Action::Scroll {
            axis: ScrollAxis::Vertical,
            amount: 0,
        };
        assert_eq!(
            bad_scroll.validate(&macro_names()),
            Err(ActionError::InvalidScrollAmount(0))
        );
    }

    #[test]
    fn validation_checks_macro_names() {
        let unknown = Action::Macro {
            name: "missing".to_owned(),
        };
        assert_eq!(
            unknown.validate(&macro_names()),
            Err(ActionError::UnknownMacro("missing".to_owned()))
        );

        let empty = Action::Macro {
            name: "  ".to_owned(),
        };
        assert_eq!(
            empty.validate(&macro_names()),
            Err(ActionError::EmptyMacroName)
        );

        let known = Action::Macro {
            name: "copy".to_owned(),
        };
        assert_eq!(known.validate(&macro_names()), Ok(()));
    }

    #[test]
    fn validation_checks_launch_path() {
        let empty = Action::Launch {
            path: PathBuf::new(),
        };
        assert_eq!(
            empty.validate(&macro_names()),
            Err(ActionError::EmptyLaunchPath)
        );
    }
}
