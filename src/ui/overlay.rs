//! Radial wheel drawn inside the root window when the overlay is active.
//!
//! When the overlay opens, the application shell morphs the root viewport into
//! a small, undecorated, always-on-top window. The wgpu renderer clears with
//! alpha=0, so pixels outside the wheel are fully transparent. This module
//! only paints the wheel geometry; it does not touch window state.

use crate::core::{ActionSet, WheelGeometry};
use crate::ui::{icons, theme};
use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Vec2};

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

/// Draws the wheel and records the outcome in `state`.
///
/// Called on the root context while the window is morphed to overlay size.
/// Paints with a transparent background so only the wheel geometry is visible.
pub fn draw(ctx: &egui::Context, sets: &[ActionSet], state: &mut OverlayState) {
    // Override visuals so the panel fill is transparent; the wheel paints
    // only its own circular geometry on top of the compositor's alpha=0 clear.
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = Color32::TRANSPARENT;
    visuals.override_text_color = Some(Color32::WHITE);
    ctx.set_visuals(visuals);

    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        state.outcome = Some(OverlayOutcome::Close);
    }

    // Focus-loss close: track when the window has had focus at least once,
    // then close if focus moves away.
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
            let size = rect.width().min(rect.height()) * 0.78;
            let geometry = WheelGeometry::from_size(size);
            let center = rect.center();
            let local_origin = center - egui::vec2(geometry.center_x, geometry.center_y);

            let pointer = ctx.input(|input| input.pointer.hover_pos());
            state.hovered = pointer.and_then(|position| {
                let local = position - local_origin;
                geometry.sector_at(sets.len(), local.x, local.y)
            });

            if ctx.input(|input| input.pointer.primary_clicked()) {
                state.outcome = Some(pointer.map_or(OverlayOutcome::Close, |position| {
                    let local = position - local_origin;
                    match geometry.sector_at(sets.len(), local.x, local.y) {
                        Some(index) => OverlayOutcome::Confirm(index),
                        None if geometry.is_center(local.x, local.y) => {
                            OverlayOutcome::Confirm(state.selected)
                        }
                        None => OverlayOutcome::Close,
                    }
                }));
            }

            paint_wheel(
                ui.painter(),
                center,
                &geometry,
                sets,
                Some(state.selected),
                state.hovered,
            );

            // Label below the wheel, with generous spacing.
            let text_y = center.y + geometry.outer_radius + 22.0;
            painter_titles(ui, sets, state, text_y, rect.center().x);
        });
}

/// Paints the unified radial wheel shared by the on-screen overlay and settings preview.
pub fn paint_wheel(
    painter: &egui::Painter,
    center: Pos2,
    geometry: &WheelGeometry,
    sets: &[ActionSet],
    selected: Option<usize>,
    hovered: Option<usize>,
) {
    let base = Color32::from_rgb(24, 28, 38);
    let hovered_fill = Color32::from_rgb(38, 44, 58);

    if sets.is_empty() {
        painter.circle_filled(center, geometry.outer_radius, base);
        return;
    }

    for (index, set) in sets.iter().enumerate() {
        let Some((start, end)) = geometry.sector_angles(index, sets.len()) else {
            continue;
        };
        let is_selected = selected == Some(index);
        let fill = if is_selected {
            theme::ACCENT
        } else if hovered == Some(index) {
            hovered_fill
        } else {
            base
        };

        paint_sector_mesh(
            painter,
            center,
            geometry.inner_radius,
            geometry.outer_radius,
            start,
            end,
            fill,
        );

        let mid = (start + end) / 2.0;
        let icon_radius = (geometry.inner_radius + geometry.outer_radius) / 2.0;
        let icon_center = center + Vec2::angled(mid) * icon_radius;
        let icon_size = (geometry.outer_radius - geometry.inner_radius) * 0.45;
        let icon_rect =
            Rect::from_center_size(icon_center, Vec2::splat(icon_size.clamp(20.0, 32.0)));
        let color = if is_selected {
            Color32::WHITE
        } else {
            Color32::from_white_alpha(220)
        };
        icons::paint(painter, icon_rect, set.icon, color);
    }

    // Center disc cleanly meeting the sector inner radius
    painter.circle_filled(center, geometry.inner_radius, base);
    if let Some(set) = selected.and_then(|i| sets.get(i)).or_else(|| sets.first()) {
        let center_icon_size = geometry.inner_radius * 1.1;
        icons::paint(
            painter,
            Rect::from_center_size(center, Vec2::splat(center_icon_size)),
            set.icon,
            Color32::WHITE,
        );
    }
}

/// Paints an annular sector using a quad-strip triangle mesh.
///
/// Avoids naive convex polygon triangulation which creates artifacts on concave annular arcs.
pub fn paint_sector_mesh(
    painter: &egui::Painter,
    center: Pos2,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    end_angle: f32,
    fill_color: Color32,
) {
    if start_angle >= end_angle {
        return;
    }
    let sweep = end_angle - start_angle;
    let steps = ((sweep / 0.08).ceil().max(4.0)) as usize;

    let mut mesh = egui::Mesh::default();
    mesh.reserve_triangles(steps * 2);
    mesh.reserve_vertices((steps + 1) * 2);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle + sweep * t;
        let dir = Vec2::angled(angle);

        let outer_pos = center + dir * outer_radius;
        let inner_pos = center + dir * inner_radius;

        mesh.colored_vertex(outer_pos, fill_color);
        mesh.colored_vertex(inner_pos, fill_color);
    }

    for i in 0..steps as u32 {
        let outer_curr = 2 * i;
        let inner_curr = 2 * i + 1;
        let outer_next = 2 * (i + 1);
        let inner_next = 2 * (i + 1) + 1;

        mesh.add_triangle(outer_curr, outer_next, inner_next);
        mesh.add_triangle(outer_curr, inner_next, inner_curr);
    }

    painter.add(egui::Shape::mesh(mesh));
}

/// Writes the selected set name below the wheel.
fn painter_titles(ui: &egui::Ui, sets: &[ActionSet], state: &OverlayState, y: f32, x: f32) {
    let painter = ui.painter();
    if let Some(set) = sets.get(state.selected).or_else(|| sets.first()) {
        painter.text(
            Pos2::new(x, y),
            Align2::CENTER_CENTER,
            truncate(&set.name, 32),
            FontId::new(16.0, egui::FontFamily::Name("HeadingSemiBold".into())),
            Color32::WHITE,
        );
    }
}

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
