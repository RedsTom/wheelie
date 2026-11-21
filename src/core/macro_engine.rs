//! Typed macros that can be replayed from the wheel.
//!
//! Replay is cooperative: it never steals a thread. Each delay is split into
//! ten-millisecond slices so cancellation is observed promptly, and the only
//! blocking call in this module is the sleep between those slices. The macro
//! language is a closed set of typed steps; unknown steps are rejected when a
//! document is read instead of being guessed at replay time.

use crate::core::action::{MouseButton, ScrollAxis};
use crate::core::key::KeyChord;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
        validate_steps(&self.steps)
    }
}

/// Checks a sequence of steps before it is stored or replayed.
///
/// A `Press` must be matched by a later `Release` of the same chord, and a
/// `Release` must follow a `Press`. A chord must not be pressed twice without
/// being released in between. This keeps a cancelled replay from leaving a key
/// stuck down on the user's desktop.
pub fn validate_steps(steps: &[MacroStep]) -> Result<(), MacroError> {
    if steps.is_empty() {
        return Err(MacroError::EmptySteps);
    }
    if steps.len() > MAX_STEPS {
        return Err(MacroError::TooManySteps(steps.len()));
    }

    let mut total_delay = 0_u64;
    let mut held = HashSet::new();
    for step in steps {
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
                if text.is_empty() {
                    return Err(MacroError::EmptyText);
                }
                if text.len() > MAX_TEXT_LEN {
                    return Err(MacroError::TextTooLong(text.len()));
                }
            }
            MacroStep::Tap { .. } | MacroStep::MouseClick { .. } => {}
            MacroStep::Press { chord } => {
                if !held.insert(*chord) {
                    return Err(MacroError::UnbalancedPressRelease(*chord));
                }
            }
            MacroStep::Release { chord } => {
                if !held.remove(chord) {
                    return Err(MacroError::UnbalancedPressRelease(*chord));
                }
            }
        }
    }

    if let Some(chord) = held.into_iter().min_by_key(|chord| chord.to_string()) {
        return Err(MacroError::UnbalancedPressRelease(chord));
    }
    if total_delay > MAX_TOTAL_DELAY_MS {
        return Err(MacroError::TooLong);
    }

    Ok(())
}

/// Maximum number of steps in one macro.
pub const MAX_STEPS: usize = 512;

/// Maximum delay for a single step, in milliseconds.
pub const MAX_DELAY_MS: u64 = 60_000;

/// Maximum combined delay for one macro, in milliseconds.
pub const MAX_TOTAL_DELAY_MS: u64 = 300_000;

/// Maximum scroll amount for one step.
pub const MAX_SCROLL_AMOUNT: u32 = 100;

/// Maximum length of one text step, in bytes.
pub const MAX_TEXT_LEN: usize = 4096;

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
    #[error(
        "scroll amount must be non-zero and at most {max}, got {0}",
        max = MAX_SCROLL_AMOUNT
    )]
    InvalidScroll(i32),

    /// A text step is empty.
    #[error("text step cannot be empty")]
    EmptyText,

    /// A text step exceeds the stored length limit.
    #[error("text step cannot exceed {max} bytes, got {0}", max = MAX_TEXT_LEN)]
    TextTooLong(usize),

    /// A `Press` and `Release` pair is not balanced.
    #[error("press and release must be paired for {0}")]
    UnbalancedPressRelease(KeyChord),

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
///
/// Steps are validated before anything is injected. Every chord still held at
/// the end is released, whether replay finished, was cancelled or failed in the
/// sink, so a key is never left stuck down.
pub fn run_macro<S: MacroSink>(
    sink: &mut S,
    steps: &[MacroStep],
    cancel: &AtomicBool,
) -> Result<(), MacroError> {
    validate_steps(steps)?;

    let mut held = HashSet::new();
    let outcome = replay(steps, sink, cancel, &mut held);

    // Release on every exit path. A cleanup failure never hides the original
    // replay error, but it is reported when replay itself succeeded.
    let mut remaining: Vec<KeyChord> = held.into_iter().collect();
    remaining.sort_by_key(|chord| chord.to_string());
    let mut cleanup_error = None;
    for chord in remaining {
        if let Err(error) = sink.release(chord)
            && cleanup_error.is_none()
        {
            cleanup_error = Some(error);
        }
    }

    match outcome {
        Ok(()) => cleanup_error.map_or(Ok(()), Err),
        Err(error) => Err(error),
    }
}

fn replay<S: MacroSink>(
    steps: &[MacroStep],
    sink: &mut S,
    cancel: &AtomicBool,
    held: &mut HashSet<KeyChord>,
) -> Result<(), MacroError> {
    for step in steps {
        if cancel.load(Ordering::Relaxed) {
            return Err(MacroError::Cancelled);
        }

        match step {
            MacroStep::Tap { chord } => sink.tap(*chord)?,
            MacroStep::Press { chord } => {
                sink.press(*chord)?;
                held.insert(*chord);
            }
            MacroStep::Release { chord } => {
                sink.release(*chord)?;
                held.remove(chord);
            }
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

    /// Sets the cancel flag from inside `press`, so cancellation happens after
    /// a chord has been held without spawning a thread.
    struct CancellingSink<'a> {
        cancel: &'a AtomicBool,
        events: Vec<String>,
    }

    impl MacroSink for CancellingSink<'_> {
        fn tap(&mut self, chord: KeyChord) -> Result<(), MacroError> {
            self.events.push(format!("tap {chord}"));
            Ok(())
        }

        fn press(&mut self, chord: KeyChord) -> Result<(), MacroError> {
            self.events.push(format!("press {chord}"));
            self.cancel.store(true, Ordering::Relaxed);
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

    /// Fails on a text step, to exercise cleanup after a sink error.
    #[derive(Default)]
    struct FailingSink {
        events: Vec<String>,
    }

    impl MacroSink for FailingSink {
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

        fn text(&mut self, _text: &str) -> Result<(), MacroError> {
            Err(MacroError::Sink("text injection failed".to_owned()))
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

    fn held_chord() -> KeyChord {
        KeyChord::with_modifiers(
            KeyCode::F13,
            Modifiers {
                control: true,
                ..Default::default()
            },
        )
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

    #[test]
    fn validation_rejects_unbalanced_press_release() {
        let chord = held_chord();
        let orphan_release = MacroDefinition {
            name: "release".to_owned(),
            steps: vec![MacroStep::Release { chord }],
        };
        assert_eq!(
            orphan_release.validate(),
            Err(MacroError::UnbalancedPressRelease(chord))
        );

        let never_released = MacroDefinition {
            name: "press".to_owned(),
            steps: vec![MacroStep::Press { chord }],
        };
        assert_eq!(
            never_released.validate(),
            Err(MacroError::UnbalancedPressRelease(chord))
        );

        let double_press = MacroDefinition {
            name: "double".to_owned(),
            steps: vec![MacroStep::Press { chord }, MacroStep::Press { chord }],
        };
        assert_eq!(
            double_press.validate(),
            Err(MacroError::UnbalancedPressRelease(chord))
        );

        let balanced = MacroDefinition {
            name: "balanced".to_owned(),
            steps: vec![MacroStep::Press { chord }, MacroStep::Release { chord }],
        };
        assert_eq!(balanced.validate(), Ok(()));
    }

    #[test]
    fn validation_allows_whitespace_text_but_rejects_overlong_text() {
        let whitespace = MacroDefinition {
            name: "space".to_owned(),
            steps: vec![MacroStep::Text {
                text: " ".to_owned(),
            }],
        };
        assert_eq!(whitespace.validate(), Ok(()));

        let overlong = MacroDefinition {
            name: "long".to_owned(),
            steps: vec![MacroStep::Text {
                text: "a".repeat(MAX_TEXT_LEN + 1),
            }],
        };
        assert_eq!(
            overlong.validate(),
            Err(MacroError::TextTooLong(MAX_TEXT_LEN + 1))
        );
    }

    #[test]
    fn scroll_limit_message_matches_the_shared_constant() {
        let too_large = MAX_SCROLL_AMOUNT as i32 + 1;
        let error = MacroError::InvalidScroll(too_large);
        assert_eq!(
            error.to_string(),
            format!(
                "scroll amount must be non-zero and at most {MAX_SCROLL_AMOUNT}, got {too_large}"
            )
        );
    }

    #[test]
    fn runner_validates_steps_before_injecting_anything() {
        let mut sink = RecordingSink::default();
        let cancel = AtomicBool::new(false);
        let steps = [MacroStep::Text {
            text: String::new(),
        }];

        assert_eq!(
            run_macro(&mut sink, &steps, &cancel),
            Err(MacroError::EmptyText)
        );
        assert!(sink.events.is_empty());
    }

    #[test]
    fn runner_releases_held_chords_on_cancel() {
        let chord = held_chord();
        let cancel = AtomicBool::new(false);
        let mut sink = CancellingSink {
            cancel: &cancel,
            events: Vec::new(),
        };
        // Balanced on paper, so validation passes: the sink sets the cancel
        // flag on press and the delay is the first step that observes it.
        let steps = [
            MacroStep::Press { chord },
            MacroStep::Delay { ms: 100 },
            MacroStep::Release { chord },
        ];

        assert_eq!(
            run_macro(&mut sink, &steps, &cancel),
            Err(MacroError::Cancelled)
        );
        assert_eq!(
            sink.events,
            vec![format!("press {chord}"), format!("release {chord}"),]
        );
    }

    #[test]
    fn runner_releases_held_chords_on_sink_error() {
        let chord = held_chord();
        let mut sink = FailingSink::default();
        let cancel = AtomicBool::new(false);
        // Balanced on paper: the sink fails on the text step, before the
        // explicit release, so cleanup has to put the chord back up.
        let steps = [
            MacroStep::Press { chord },
            MacroStep::Text {
                text: "hello".to_owned(),
            },
            MacroStep::Release { chord },
        ];

        assert_eq!(
            run_macro(&mut sink, &steps, &cancel),
            Err(MacroError::Sink("text injection failed".to_owned()))
        );
        assert_eq!(
            sink.events,
            vec![format!("press {chord}"), format!("release {chord}")]
        );
    }

    #[test]
    fn runner_releases_held_chords_on_success() {
        let chord = held_chord();
        let mut sink = RecordingSink::default();
        let cancel = AtomicBool::new(false);
        let steps = [MacroStep::Press { chord }, MacroStep::Release { chord }];

        run_macro(&mut sink, &steps, &cancel).expect("run");
        assert_eq!(
            sink.events,
            vec![format!("press {chord}"), format!("release {chord}")]
        );
    }
}
