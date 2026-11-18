//! Radial wheel drawn in its own transparent, always-on-top viewport.
//!
//! The wheel shows one sector per action set, with its icon. Confirming a
//! sector makes that set the active one; the wheel never runs an action
//! directly.

use crate::core::{ActionSet, WheelGeometry};
use crate::ui::icons;
use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Size of the overlay viewport, in points.
pub const OVERLAY_SIZE: f32 = 340.0;

/// Margin kept between the wheel and the viewport edges.
const WHEEL_MARGIN: f32 = 44.0;

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

/// Draws the wheel and records the outcome in `state`.
pub fn draw(ctx: &egui::Context, sets: &[ActionSet], state: &mut OverlayState) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = Color32::TRANSPARENT;
    visuals.override_text_color = Some(Color32::WHITE);
    ctx.set_visuals(visuals);

    if let Some(command) = egui::ViewportCommand::center_on_screen(ctx) {
        ctx.send_viewport_cmd(command);
    }

    if ctx.input(|input| input.key_pressed(egui::Key::Escape))
        || ctx.input(|input| input.viewport().close_requested())
    {
        state.outcome = Some(OverlayOutcome::Close);
    }

    // Clicking another window (or anything that takes focus) closes the wheel.
    let focused = ctx.input(|input| input.viewport().focused).unwrap_or(true);
    if focused {
        state.focused_once = true;
    } else if state.focused_once {
        state.outcome = Some(OverlayOutcome::Close);
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(Color32::TRANSPARENT))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let size = (rect.width().min(rect.height()) - WHEEL_MARGIN).max(120.0);
            let square = Rect::from_center_size(rect.center(), Vec2::splat(size));
            let geometry = WheelGeometry::from_size(size);
            let center = square.min + egui::vec2(geometry.center_x, geometry.center_y);

            let pointer = ctx.input(|input| input.pointer.hover_pos());
            state.hovered = pointer.and_then(|position| {
                let local = position - square.min;
                geometry.sector_at(sets.len(), local.x, local.y)
            });

            if ctx.input(|input| input.pointer.primary_clicked()) {
                state.outcome = Some(pointer.map_or(OverlayOutcome::Close, |position| {
                    let local = position - square.min;
                    match geometry.sector_at(sets.len(), local.x, local.y) {
                        Some(index) => OverlayOutcome::Confirm(index),
                        None if geometry.is_center(local.x, local.y) => {
                            OverlayOutcome::Confirm(state.selected)
                        }
                        None => OverlayOutcome::Close,
                    }
                }));
            }

            paint(ui, sets, state, &geometry, center, size);

            painter_titles(ui, sets, state, rect);
        });
}

fn paint(
    ui: &egui::Ui,
    sets: &[ActionSet],
    state: &OverlayState,
    geometry: &WheelGeometry,
    center: Pos2,
    size: f32,
) {
    let painter = ui.painter();
    let base = Color32::from_rgba_unmultiplied(24, 28, 38, 235);
    let hovered_fill = Color32::from_rgba_unmultiplied(38, 44, 58, 245);
    let accent = Color32::from_rgb(75, 139, 255);

    if sets.is_empty() {
        painter.circle_filled(center, geometry.outer_radius, base);
        return;
    }

    let step = std::f32::consts::TAU / sets.len() as f32;
    let gap = (step * 0.045).min(0.035);

    for (index, set) in sets.iter().enumerate() {
        let Some((start, end)) = geometry.sector_angles(index, sets.len()) else {
            continue;
        };
        let selected = state.selected == index;
        let hovered = state.hovered == Some(index);
        let fill = if selected {
            accent
        } else if hovered {
            hovered_fill
        } else {
            base
        };

        let points = sector_points(center, geometry, start + gap, end - gap);
        painter.add(egui::Shape::convex_polygon(points, fill, Stroke::NONE));

        let mid = (start + end) / 2.0;
        let icon_radius = (geometry.inner_radius + geometry.outer_radius) / 2.0;
        let icon_center = center + Vec2::angled(mid) * icon_radius;
        let icon_side = (size * 0.09).clamp(20.0, 32.0);
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(icon_side));
        let color = if selected {
            Color32::WHITE
        } else {
            Color32::from_white_alpha(220)
        };
        icons::paint(painter, icon_rect, set.icon, color);
    }

    painter.circle_filled(center, geometry.inner_radius * 0.92, base);
    if let Some(set) = sets.get(state.selected).or_else(|| sets.first()) {
        let icon_rect = Rect::from_center_size(center, Vec2::splat(geometry.inner_radius * 1.15));
        icons::paint(painter, icon_rect, set.icon, Color32::WHITE);
    }
}

/// Writes the selected set name once, below the wheel.
fn painter_titles(ui: &egui::Ui, sets: &[ActionSet], state: &OverlayState, rect: Rect) {
    let painter = ui.painter();
    if let Some(set) = sets.get(state.selected).or_else(|| sets.first()) {
        painter.text(
            rect.center_bottom() - egui::vec2(0.0, 14.0),
            Align2::CENTER_BOTTOM,
            truncate(&set.name, 32),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
    }
}

fn sector_points(center: Pos2, geometry: &WheelGeometry, start: f32, end: f32) -> Vec<Pos2> {
    let steps = ((end - start) / 0.12).ceil().max(4.0) as usize;
    let mut points = Vec::with_capacity((steps + 1) * 2);
    for index in 0..=steps {
        let angle = start + (end - start) * index as f32 / steps as f32;
        points.push(center + Vec2::angled(angle) * geometry.outer_radius);
    }
    for index in (0..=steps).rev() {
        let angle = start + (end - start) * index as f32 / steps as f32;
        points.push(center + Vec2::angled(angle) * geometry.inner_radius);
    }
    points
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut truncated: String = text.chars().take(limit.saturating_sub(1)).collect();
    truncated.push('…');
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
        assert!(truncated.ends_with('…'));
    }
}
