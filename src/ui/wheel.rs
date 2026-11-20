//! State of the radial wheel, shared by the overlay window, the settings
//! preview and the shell that drives it.
//!
//! The wheel is a selection surface only: it never runs an action. The paint
//! code lives in [`crate::ui::wheel_paint`], the geometry in
//! [`crate::core::WheelGeometry`].

/// Size of the overlay window in points (width = height).
pub const OVERLAY_SIZE: f32 = 380.0;

/// What the user did in the overlay this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayOutcome {
    /// The overlay should close without changing the active set.
    Close,
    /// Make the set at this index the active one.
    Confirm(usize),
}

/// Transient overlay state owned by the application shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverlayState {
    /// Whether the overlay is currently shown.
    pub open: bool,
    /// Knob-driven selection.
    pub selected: usize,
    /// Sector currently under the mouse.
    pub hovered: Option<usize>,
    /// Outcome of the last frame, consumed by the shell.
    pub outcome: Option<OverlayOutcome>,
    /// True once the overlay has seen its own focus, so losing focus later
    /// closes it without closing on the very first frame.
    pub focused_once: bool,
}

impl OverlayState {
    /// Opens the overlay on the active set.
    pub fn open_on(&mut self, index: usize, count: usize) {
        self.open = true;
        self.selected = index.min(count.saturating_sub(1));
        self.hovered = None;
        self.outcome = None;
        self.focused_once = false;
    }

    /// Moves the knob selection one set to the left.
    pub fn select_previous(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = (self.selected + count - 1) % count;
    }

    /// Moves the knob selection one set to the right.
    pub fn select_next(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % count;
    }
}

/// Shortens a set name so it fits the wheel label slot.
pub(crate) fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut truncated: String = text.chars().take(limit.saturating_sub(1)).collect();
    truncated.push('.');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_cycles_through_the_sets() {
        let mut state = OverlayState::default();
        state.open_on(1, 3);
        assert_eq!(state.selected, 1);

        state.select_next(3);
        assert_eq!(state.selected, 2);
        state.select_next(3);
        assert_eq!(state.selected, 0);
        state.select_previous(3);
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn empty_wheel_ignores_navigation() {
        let mut state = OverlayState::default();
        state.open_on(0, 0);
        state.select_next(0);
        state.select_previous(0);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn opening_on_a_stale_index_is_clamped() {
        let mut state = OverlayState::default();
        state.open_on(9, 3);
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn long_labels_are_truncated() {
        assert_eq!(truncate("short", 10), "short");
        let truncated = truncate("a very long action set name", 10);
        assert_eq!(truncated.chars().count(), 10);
        assert!(truncated.ends_with('.'));
    }
}
