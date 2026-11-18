//! Radial wheel drawn in its own transparent, always-on-top viewport.
//!
//! The wheel shows one sector per action set, with its icon. Confirming a
//! sector makes that set the active one; the wheel never runs an action
//! directly.

use crate::core::{ActionSet, Language, TextKey, WheelGeometry, tr};
use crate::ui::icons;
use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Size of the overlay viewport, in points.
pub const OVERLAY_SIZE: f32 = 460.0;

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
}

impl OverlayState {
    /// Opens the overlay on the active set.
    pub fn open_on(&mut self, index: usize, count: usize) {
        self.open = true;
        self.selected = index.min(count.saturating_sub(1));
        self.hovered = None;
        self.outcome = None;
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

/// Builder for the overlay viewport.
pub fn viewport_builder() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_inner_size([OVERLAY_SIZE, OVERLAY_SIZE])
        .with_min_inner_size([OVERLAY_SIZE, OVERLAY_SIZE])
        .with_max_inner_size([OVERLAY_SIZE, OVERLAY_SIZE])
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_active(true)
        .with_title("Wheelie")
}

/// Draws the wheel and records the outcome in `state`.
pub fn draw(ctx: &egui::Context, language: Language, sets: &[ActionSet], state: &mut OverlayState) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = Color32::TRANSPARENT;
    visuals.override_text_color = Some(Color32::WHITE);
    ctx.set_visuals(visuals);

    if let Some(command) = egui::ViewportCommand::center_on_screen(ctx) {
        ctx.send_viewport_cmd(command);
    }

    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        state.outcome = Some(OverlayOutcome::Close);
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(Color32::TRANSPARENT))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let size = rect.width().min(rect.height());
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

            painter_hint(ui, rect, language);
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
    let border = Stroke::new(1.5_f32, Color32::from_white_alpha(46));
    let base = Color32::from_rgba_unmultiplied(22, 26, 36, 238);
    let accent = Color32::from_rgba_unmultiplied(75, 139, 255, 242);

    if sets.is_empty() {
        painter.circle_filled(center, geometry.outer_radius, base);
        painter.circle_stroke(center, geometry.outer_radius, border);
    }

    for (index, set) in sets.iter().enumerate() {
        let Some((start, end)) = geometry.sector_angles(index, sets.len()) else {
            continue;
        };
        let selected = state.selected == index;
        let hovered = state.hovered == Some(index);
        let fill = if selected || hovered { accent } else { base };

        let points = sector_points(center, geometry, start, end);
        painter.add(egui::Shape::convex_polygon(points, fill, border));

        let mid = (start + end) / 2.0;
        let icon_radius = (geometry.inner_radius + geometry.outer_radius) / 2.0;
        let icon_center = center + Vec2::angled(mid) * icon_radius;
        let icon_side = (size * 0.075).clamp(18.0, 34.0);
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(icon_side));
        icons::paint(painter, icon_rect, set.icon, Color32::WHITE);

        let label_radius = icon_radius + icon_side * 0.9;
        let label_position = center + Vec2::angled(mid) * label_radius;
        painter.text(
            label_position,
            Align2::CENTER_CENTER,
            truncate(&set.name, label_capacity(sets.len())),
            FontId::proportional(11.5),
            Color32::WHITE,
        );
    }

    painter.circle_filled(center, geometry.inner_radius, accent);
    painter.circle_stroke(center, geometry.inner_radius, border);
    if let Some(set) = sets.get(state.selected).or_else(|| sets.first()) {
        let icon_rect = Rect::from_center_size(center, Vec2::splat(geometry.inner_radius * 1.1));
        icons::paint(painter, icon_rect, set.icon, Color32::WHITE);
    }
}

fn painter_hint(ui: &egui::Ui, rect: Rect, language: Language) {
    ui.painter().text(
        rect.center_bottom() - egui::vec2(0.0, 18.0),
        Align2::CENTER_BOTTOM,
        tr(language, TextKey::OverlayHint),
        FontId::proportional(11.0),
        Color32::from_white_alpha(170),
    );
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

fn label_capacity(count: usize) -> usize {
    match count {
        0..=3 => 26,
        4..=6 => 18,
        _ => 12,
    }
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
