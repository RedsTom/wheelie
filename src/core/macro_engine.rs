//! Typed macros that can be replayed from the wheel.

use crate::core::action::{MouseButton, ScrollAxis};
use crate::core::key::KeyChord;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use thiserror::Error;

/// One step of a macro sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroStep {
    /// Press and release a chord.
    Tap {
        /// Chord to tap.
        chord: KeyChord,
    },
    /// Hold a chord down.
    Press {
        /// Chord to hold.
        chord: KeyChord,
    },
    /// Release a previously pressed chord.
    Release {
        /// Chord to release.
        chord: KeyChord,
    },
    /// Type text.
    Text {
        /// Text to type.
        text: String,
    },
    /// Wait.
    Delay {
        /// Delay in milliseconds.
        ms: u64,
    },
    /// Scroll the window under the cursor.
    Scroll {
        /// Scroll axis.
        axis: ScrollAxis,
        /// Signed amount.
        amount: i32,
    },
    /// Click a mouse button.
    MouseClick {
        /// Button to click.
        button: MouseButton,
    },
}

/// A named macro stored in the configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroDefinition {
    /// Unique macro name.
    pub name: String,
    /// Ordered steps.
    #[serde(default)]
    pub steps: Vec<MacroStep>,
}

impl MacroDefinition {
    /// Checks structural limits before the macro can be stored.
    pub fn validate(&self) -> Result<(), MacroError> {
        if self.name.trim().is_empty() {
            return Err(MacroError::EmptyName);
        }
        if self.steps.is_empty() {
            return Err(MacroError::EmptySteps);
        }
        if self.steps.len() > MAX_STEPS {
            return Err(MacroError::TooManySteps(self.steps.len()));
        }

        let mut total_delay = 0_u64;
        for step in &self.steps {
            match step {
                MacroStep::Delay { ms } => {
                    if *ms > MAX_DELAY_MS {
                        return Err(MacroError::InvalidDelay(*ms));
                    }
                    total_delay += *ms;
                }
                MacroStep::Scroll { amount, .. } => {
                    if *amount == 0 || amount.unsigned_abs() > MAX_SCROLL_AMOUNT {
                        return Err(MacroError::InvalidScroll(*amount));
                    }
                }
                MacroStep::Text { text } => {
                    if text.trim().is_empty() {
                        return Err(MacroError::EmptyText);
                    }
                }
                MacroStep::Tap { .. }
                | MacroStep::Press { .. }
                | MacroStep::Release { .. }
                | MacroStep::MouseClick { .. } => {}
            }
        }

        if total_delay > MAX_TOTAL_DELAY_MS {
            return Err(MacroError::TooLong);
        }

        Ok(())
    }
}

/// Maximum number of steps in one macro.
pub const MAX_STEPS: usize = 512;

/// Maximum delay for a single step, in milliseconds.
pub const MAX_DELAY_MS: u64 = 60_000;

/// Maximum combined delay for one macro, in milliseconds.
pub const MAX_TOTAL_DELAY_MS: u64 = 300_000;

/// Maximum scroll amount for one step.
pub const MAX_SCROLL_AMOUNT: u32 = 100;

/// Error returned by macro validation or execution.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MacroError {
    /// The macro name is empty.
    #[error("macro name cannot be empty")]
    EmptyName,

    /// The macro has no steps.
    #[error("macro must contain at least one step")]
    EmptySteps,

    /// The macro has too many steps.
    #[error("macro has too many steps: {0}")]
    TooManySteps(usize),

    /// A delay exceeds the per-step limit.
    #[error("delay must be at most 60000 ms, got {0}")]
    InvalidDelay(u64),

    /// A scroll amount is zero or too large.
    #[error("scroll amount must be non-zero and at most 100, got {0}")]
    InvalidScroll(i32),

    /// A text step is empty.
    #[error("text step cannot be empty")]
    EmptyText,

    /// Combined delays exceed the total limit.
    #[error("macro delays exceed 300000 ms")]
    TooLong,

    /// Execution was cancelled.
    #[error("macro execution was cancelled")]
    Cancelled,

    /// The platform input sink failed.
    #[error("input failed: {0}")]
    Sink(String),
}

/// Platform-agnostic sink used to replay macro steps.
///
/// The core crate only defines this trait; the platform layer implements it.
pub trait MacroSink {
    /// Press and release a chord.
    fn tap(&mut self, chord: KeyChord) -> Result<(), MacroError>;
    /// Hold a chord down.
    fn press(&mut self, chord: KeyChord) -> Result<(), MacroError>;
    /// Release a chord.
    fn release(&mut self, chord: KeyChord) -> Result<(), MacroError>;
    /// Type text.
    fn text(&mut self, text: &str) -> Result<(), MacroError>;
    /// Scroll the window under the cursor.
    fn scroll(&mut self, axis: ScrollAxis, amount: i32) -> Result<(), MacroError>;
    /// Click a mouse button.
    fn mouse_click(&mut self, button: MouseButton) -> Result<(), MacroError>;
}

/// Replays macro steps through `sink`, stopping when `cancel` is set.
pub fn run_macro<S: MacroSink>(
    sink: &mut S,
    steps: &[MacroStep],
    cancel: &AtomicBool,
) -> Result<(), MacroError> {
    for step in steps {
        if cancel.load(Ordering::Relaxed) {
            return Err(MacroError::Cancelled);
        }

        match step {
            MacroStep::Tap { chord } => sink.tap(*chord)?,
            MacroStep::Press { chord } => sink.press(*chord)?,
            MacroStep::Release { chord } => sink.release(*chord)?,
            MacroStep::Text { text } => sink.text(text)?,
            MacroStep::Delay { ms } => sleep_cancellable(Duration::from_millis(*ms), cancel)?,
            MacroStep::Scroll { axis, amount } => sink.scroll(*axis, *amount)?,
            MacroStep::MouseClick { button } => sink.mouse_click(*button)?,
        }
    }

    Ok(())
}

fn sleep_cancellable(duration: Duration, cancel: &AtomicBool) -> Result<(), MacroError> {
    const SLICE: Duration = Duration::from_millis(10);
    let mut remaining = duration;

    while remaining > Duration::ZERO {
        if cancel.load(Ordering::Relaxed) {
            return Err(MacroError::Cancelled);
        }
        let chunk = remaining.min(SLICE);
        std::thread::sleep(chunk);
        remaining -= chunk;
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(MacroError::Cancelled);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::key::{KeyCode, Modifiers};

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<String>,
    }

    impl MacroSink for RecordingSink {
        fn tap(&mut self, chord: KeyChord) -> Result<(), MacroError> {
            self.events.push(format!("tap {chord}"));
            Ok(())
        }

        fn press(&mut self, chord: KeyChord) -> Result<(), MacroError> {
            self.events.push(format!("press {chord}"));
            Ok(())
        }

        fn release(&mut self, chord: KeyChord) -> Result<(), MacroError> {
            self.events.push(format!("release {chord}"));
            Ok(())
        }

        fn text(&mut self, text: &str) -> Result<(), MacroError> {
            self.events.push(format!("text {text}"));
            Ok(())
        }

        fn scroll(&mut self, axis: ScrollAxis, amount: i32) -> Result<(), MacroError> {
            self.events.push(format!("scroll {axis:?} {amount}"));
            Ok(())
        }

        fn mouse_click(&mut self, button: MouseButton) -> Result<(), MacroError> {
            self.events.push(format!("click {button:?}"));
            Ok(())
        }
    }

    fn copy_macro() -> MacroDefinition {
        MacroDefinition {
            name: "copy".to_owned(),
            steps: vec![
                MacroStep::Tap {
                    chord: KeyChord::with_modifiers(
                        KeyCode::KeyC,
                        Modifiers {
                            control: true,
                            ..Default::default()
                        },
                    ),
                },
                MacroStep::Delay { ms: 0 },
                MacroStep::Text {
                    text: "hello".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn validation_accepts_a_valid_macro() {
        assert_eq!(copy_macro().validate(), Ok(()));
    }

    #[test]
    fn validation_rejects_empty_name_and_steps() {
        let empty_name = MacroDefinition {
            name: " ".to_owned(),
            steps: vec![MacroStep::Delay { ms: 0 }],
        };
        assert_eq!(empty_name.validate(), Err(MacroError::EmptyName));

        let empty_steps = MacroDefinition {
            name: "copy".to_owned(),
            steps: vec![],
        };
        assert_eq!(empty_steps.validate(), Err(MacroError::EmptySteps));
    }

    #[test]
    fn validation_rejects_bad_delays_and_scrolls() {
        let long_delay = MacroDefinition {
            name: "slow".to_owned(),
            steps: vec![MacroStep::Delay {
                ms: MAX_DELAY_MS + 1,
            }],
        };
        assert_eq!(
            long_delay.validate(),
            Err(MacroError::InvalidDelay(MAX_DELAY_MS + 1))
        );

        let bad_scroll = MacroDefinition {
            name: "scroll".to_owned(),
            steps: vec![MacroStep::Scroll {
                axis: ScrollAxis::Vertical,
                amount: 0,
            }],
        };
        assert_eq!(bad_scroll.validate(), Err(MacroError::InvalidScroll(0)));
    }

    #[test]
    fn runner_replays_steps_in_order() {
        let macro_definition = copy_macro();
        let mut sink = RecordingSink::default();
        let cancel = AtomicBool::new(false);

        run_macro(&mut sink, &macro_definition.steps, &cancel).expect("run");

        assert_eq!(
            sink.events,
            vec!["tap Ctrl+C".to_owned(), "text hello".to_owned()]
        );
    }

    #[test]
    fn runner_stops_when_cancelled() {
        let macro_definition = copy_macro();
        let mut sink = RecordingSink::default();
        let cancel = AtomicBool::new(true);

        assert_eq!(
            run_macro(&mut sink, &macro_definition.steps, &cancel),
            Err(MacroError::Cancelled)
        );
        assert!(sink.events.is_empty());
    }

    #[test]
    fn macro_serde_round_trips() {
        let macro_definition = copy_macro();
        let serialized = toml::to_string(&macro_definition).expect("serialize");
        let deserialized: MacroDefinition = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, macro_definition);
    }
}
