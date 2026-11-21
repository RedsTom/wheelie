//! Gesture detection for knob rotation and press.
//!
//! The default timings declared here are the single source of truth for both
//! gesture detection and persisted configuration.

use crate::core::key::KeyChord;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Physical knob control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Control {
    /// Rotate the knob to the left.
    Left,
    /// Rotate the knob to the right.
    Right,
    /// Press the knob.
    Press,
}

/// Event produced by the gesture detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureEvent {
    /// The knob was rotated left.
    RotateLeft,
    /// The knob was rotated right.
    RotateRight,
    /// The knob was pressed once.
    SinglePress,
    /// The knob was pressed twice quickly.
    DoublePress,
}

/// Normalized input event from the platform layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// A bound key went down.
    Pressed(KeyChord),
    /// A bound key went up.
    Released(KeyChord),
}

/// The three key chords that drive one profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bindings {
    /// Key for left rotation.
    pub left: KeyChord,
    /// Key for right rotation.
    pub right: KeyChord,
    /// Key for knob press.
    pub press: KeyChord,
}

impl Bindings {
    /// Checks that the three chords are distinct.
    pub fn validate(&self) -> Result<(), BindingsError> {
        if self.left == self.right {
            return Err(BindingsError::LeftEqualsRight);
        }
        if self.left == self.press {
            return Err(BindingsError::LeftEqualsPress);
        }
        if self.right == self.press {
            return Err(BindingsError::RightEqualsPress);
        }
        Ok(())
    }

    /// Maps a chord to its control, if it is bound.
    pub fn control_for(&self, chord: KeyChord) -> Option<Control> {
        if chord == self.left {
            Some(Control::Left)
        } else if chord == self.right {
            Some(Control::Right)
        } else if chord == self.press {
            Some(Control::Press)
        } else {
            None
        }
    }

    /// Returns the chord bound to a control.
    pub fn chord_for(&self, control: Control) -> KeyChord {
        match control {
            Control::Left => self.left,
            Control::Right => self.right,
            Control::Press => self.press,
        }
    }
}

/// Error returned when bindings are ambiguous.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindingsError {
    /// Left and right use the same chord.
    #[error("left and right bindings must be different")]
    LeftEqualsRight,

    /// Left and press use the same chord.
    #[error("left and press bindings must be different")]
    LeftEqualsPress,

    /// Right and press use the same chord.
    #[error("right and press bindings must be different")]
    RightEqualsPress,
}

/// Default maximum gap between two presses, in milliseconds.
pub const DEFAULT_DOUBLE_CLICK_MS: u64 = 350;

/// Default minimum gap between two rotation steps, in milliseconds.
pub const DEFAULT_ROTATION_DEBOUNCE_MS: u64 = 20;

/// Timing settings for gesture detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GestureConfig {
    /// Maximum gap between two presses to count as a double press.
    pub double_click_ms: u64,
    /// Minimum gap between two rotation steps.
    pub rotation_debounce_ms: u64,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            double_click_ms: DEFAULT_DOUBLE_CLICK_MS,
            rotation_debounce_ms: DEFAULT_ROTATION_DEBOUNCE_MS,
        }
    }
}

impl GestureConfig {
    /// Checks timing limits.
    pub fn validate(&self) -> Result<(), GestureConfigError> {
        if !(100..=1_000).contains(&self.double_click_ms) {
            return Err(GestureConfigError::InvalidDoubleClick(self.double_click_ms));
        }
        if self.rotation_debounce_ms > 500 {
            return Err(GestureConfigError::InvalidDebounce(
                self.rotation_debounce_ms,
            ));
        }
        Ok(())
    }
}

/// Error returned for invalid gesture timing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GestureConfigError {
    /// Double-click window is outside the supported range.
    #[error("double click delay must be between 100 and 1000 ms, got {0}")]
    InvalidDoubleClick(u64),

    /// Rotation debounce is too large.
    #[error("rotation debounce must be at most 500 ms, got {0}")]
    InvalidDebounce(u64),
}

/// Converts raw key events into knob gestures.
pub struct GestureDetector {
    bindings: Bindings,
    config: GestureConfig,
    pressed: HashSet<Control>,
    pending_press: Option<(Control, Instant)>,
    last_rotation: HashMap<Control, Instant>,
    overlay_open: bool,
}

impl GestureDetector {
    /// Creates a detector for one profile.
    pub fn new(bindings: Bindings, config: GestureConfig) -> Self {
        Self {
            bindings,
            config,
            pressed: HashSet::new(),
            pending_press: None,
            last_rotation: HashMap::new(),
            overlay_open: false,
        }
    }

    /// Current bindings.
    pub fn bindings(&self) -> Bindings {
        self.bindings
    }

    /// Replaces the bindings, clearing pending state.
    pub fn set_bindings(&mut self, bindings: Bindings) {
        self.bindings = bindings;
        self.reset();
    }

    /// Whether the radial overlay is open.
    pub fn overlay_open(&self) -> bool {
        self.overlay_open
    }

    /// Opens or closes the overlay. Opening cancels a pending single press.
    pub fn set_overlay_open(&mut self, open: bool) {
        self.overlay_open = open;
        if open {
            self.pending_press = None;
        }
    }

    /// Feeds one input event and returns any gestures it produced.
    pub fn feed(&mut self, event: InputEvent, now: Instant) -> Vec<GestureEvent> {
        match event {
            InputEvent::Pressed(chord) => self.on_pressed(chord, now),
            InputEvent::Released(chord) => {
                if let Some(control) = self.bindings.control_for(chord) {
                    self.pressed.remove(&control);
                }
                Vec::new()
            }
        }
    }

    /// Expires a pending single press when its window has elapsed.
    pub fn tick(&mut self, now: Instant) -> Vec<GestureEvent> {
        if let Some((_, started)) = self.pending_press
            && now.duration_since(started) >= Duration::from_millis(self.config.double_click_ms)
        {
            self.pending_press = None;
            return vec![GestureEvent::SinglePress];
        }
        Vec::new()
    }

    /// Deadline of the pending single press, used to schedule a repaint.
    pub fn pending_expiry(&self) -> Option<Instant> {
        self.pending_press
            .map(|(_, started)| started + Duration::from_millis(self.config.double_click_ms))
    }

    /// Clears pressed and pending state.
    pub fn reset(&mut self) {
        self.pressed.clear();
        self.pending_press = None;
        self.last_rotation.clear();
    }

    fn on_pressed(&mut self, chord: KeyChord, now: Instant) -> Vec<GestureEvent> {
        let Some(control) = self.bindings.control_for(chord) else {
            return Vec::new();
        };

        if !self.pressed.insert(control) {
            return Vec::new();
        }

        match control {
            Control::Left | Control::Right => {
                let debounce = Duration::from_millis(self.config.rotation_debounce_ms);
                if let Some(last) = self.last_rotation.get(&control)
                    && now.duration_since(*last) < debounce
                {
                    return Vec::new();
                }
                self.last_rotation.insert(control, now);

                if control == Control::Left {
                    vec![GestureEvent::RotateLeft]
                } else {
                    vec![GestureEvent::RotateRight]
                }
            }
            Control::Press => {
                if self.overlay_open {
                    self.pending_press = None;
                    return vec![GestureEvent::SinglePress];
                }

                if let Some((pending, started)) = self.pending_press
                    && pending == Control::Press
                    && now.duration_since(started)
                        < Duration::from_millis(self.config.double_click_ms)
                {
                    self.pending_press = None;
                    return vec![GestureEvent::DoublePress];
                }

                self.pending_press = Some((Control::Press, now));
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::key::{KeyCode, Modifiers};

    fn bindings() -> Bindings {
        Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        }
    }

    fn detector() -> GestureDetector {
        GestureDetector::new(bindings(), GestureConfig::default())
    }

    #[test]
    fn bindings_reject_duplicates() {
        let duplicate = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F13),
            press: KeyChord::new(KeyCode::F15),
        };
        assert_eq!(duplicate.validate(), Err(BindingsError::LeftEqualsRight));

        let duplicate = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F13),
        };
        assert_eq!(duplicate.validate(), Err(BindingsError::LeftEqualsPress));

        let duplicate = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F14),
        };
        assert_eq!(duplicate.validate(), Err(BindingsError::RightEqualsPress));

        assert_eq!(bindings().validate(), Ok(()));
    }

    #[test]
    fn single_press_is_delayed_until_the_window_expires() {
        let mut detector = detector();
        let start = Instant::now();

        let events = detector.feed(InputEvent::Pressed(bindings().press), start);
        assert!(events.is_empty());

        let before = start + Duration::from_millis(DEFAULT_DOUBLE_CLICK_MS - 1);
        assert!(detector.tick(before).is_empty());

        let after = start + Duration::from_millis(DEFAULT_DOUBLE_CLICK_MS);
        assert_eq!(detector.tick(after), vec![GestureEvent::SinglePress]);
        assert!(detector.pending_expiry().is_none());
    }

    #[test]
    fn second_press_within_the_window_cancels_the_single_press() {
        let mut detector = detector();
        let start = Instant::now();

        detector.feed(InputEvent::Pressed(bindings().press), start);
        detector.feed(
            InputEvent::Released(bindings().press),
            start + Duration::from_millis(10),
        );
        let events = detector.feed(
            InputEvent::Pressed(bindings().press),
            start + Duration::from_millis(100),
        );

        assert_eq!(events, vec![GestureEvent::DoublePress]);
        assert!(detector.tick(start + Duration::from_millis(500)).is_empty());
    }

    #[test]
    fn rotation_is_debounced() {
        let mut detector = detector();
        let start = Instant::now();

        let events = detector.feed(InputEvent::Pressed(bindings().left), start);
        assert_eq!(events, vec![GestureEvent::RotateLeft]);
        detector.feed(
            InputEvent::Released(bindings().left),
            start + Duration::from_millis(5),
        );

        let too_fast = detector.feed(
            InputEvent::Pressed(bindings().left),
            start + Duration::from_millis(10),
        );
        assert!(too_fast.is_empty());

        detector.feed(
            InputEvent::Released(bindings().left),
            start + Duration::from_millis(12),
        );
        let after_debounce = detector.feed(
            InputEvent::Pressed(bindings().left),
            start + Duration::from_millis(25),
        );
        assert_eq!(after_debounce, vec![GestureEvent::RotateLeft]);
    }

    #[test]
    fn right_rotation_emits_its_own_event() {
        let mut detector = detector();
        let start = Instant::now();

        let events = detector.feed(InputEvent::Pressed(bindings().right), start);
        assert_eq!(events, vec![GestureEvent::RotateRight]);
    }

    #[test]
    fn press_is_immediate_while_the_overlay_is_open() {
        let mut detector = detector();
        detector.set_overlay_open(true);
        let start = Instant::now();

        let events = detector.feed(InputEvent::Pressed(bindings().press), start);
        assert_eq!(events, vec![GestureEvent::SinglePress]);
        assert!(detector.tick(start + Duration::from_millis(500)).is_empty());
    }

    #[test]
    fn unknown_chords_are_ignored() {
        let mut detector = detector();
        let start = Instant::now();

        let events = detector.feed(InputEvent::Pressed(KeyChord::new(KeyCode::F20)), start);
        assert!(events.is_empty());
    }

    #[test]
    fn repeated_press_without_release_is_ignored() {
        let mut detector = detector();
        let start = Instant::now();

        detector.feed(InputEvent::Pressed(bindings().left), start);
        let repeated = detector.feed(
            InputEvent::Pressed(bindings().left),
            start + Duration::from_millis(100),
        );
        assert!(repeated.is_empty());
    }

    #[test]
    fn release_alone_does_not_emit_a_gesture() {
        let mut detector = detector();
        let start = Instant::now();

        let events = detector.feed(InputEvent::Released(bindings().press), start);
        assert!(events.is_empty());
    }

    #[test]
    fn gesture_config_validates_ranges() {
        assert_eq!(GestureConfig::default().validate(), Ok(()));

        let too_small = GestureConfig {
            double_click_ms: 50,
            ..Default::default()
        };
        assert_eq!(
            too_small.validate(),
            Err(GestureConfigError::InvalidDoubleClick(50))
        );

        let too_slow = GestureConfig {
            rotation_debounce_ms: 501,
            ..Default::default()
        };
        assert_eq!(
            too_slow.validate(),
            Err(GestureConfigError::InvalidDebounce(501))
        );
    }

    #[test]
    fn bindings_return_the_chord_of_each_control() {
        let bindings = bindings();
        assert_eq!(bindings.chord_for(Control::Left), bindings.left);
        assert_eq!(bindings.chord_for(Control::Right), bindings.right);
        assert_eq!(bindings.chord_for(Control::Press), bindings.press);
    }

    #[test]
    fn bindings_support_modifier_chords() {
        let chord = KeyChord::with_modifiers(
            KeyCode::F13,
            Modifiers {
                control: true,
                ..Default::default()
            },
        );
        let bindings = Bindings {
            left: chord,
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        };
        assert_eq!(bindings.control_for(chord), Some(Control::Left));
    }

    #[test]
    fn default_timings_match_the_shared_constants() {
        assert_eq!(DEFAULT_DOUBLE_CLICK_MS, 350);
        assert_eq!(DEFAULT_ROTATION_DEBOUNCE_MS, 20);
        assert_eq!(
            GestureConfig::default().double_click_ms,
            DEFAULT_DOUBLE_CLICK_MS
        );
        assert_eq!(
            GestureConfig::default().rotation_debounce_ms,
            DEFAULT_ROTATION_DEBOUNCE_MS
        );
    }
}
