//! Radial wheel drawn in its own transparent, always-on-top viewport.
//!
//! The wheel uses the pure geometry from `core::wheel` for hit testing, so knob
//! and mouse navigation select the same segments.

use crate::core::profile::Actions;
use crate::core::{Control, Language, TextKey, WheelGeometry, WheelSegment, tr};
use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Size of the overlay viewport, in points.
pub const OVERLAY_SIZE: f32 = 420.0;

/// What the user did in the overlay this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayOutcome {
    /// The overlay should close without running an action.
    Close,
    /// Run the action of the selected control and close.
    Confirm(Control),
}

/// Transient overlay state owned by the application shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayState {
    /// Whether the overlay is currently shown.
    pub open: bool,
    /// Knob-driven selection.
    pub selection: Control,
    /// Segment currently under the mouse.
    pub hovered: Option<WheelSegment>,
    /// Outcome of the last frame, consumed by the shell.
    pub outcome: Option<OverlayOutcome>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            open: false,
            selection: Control::Press,
            hovered: None,
            outcome: None,
        }
    }
}

impl OverlayState {
    /// Moves the knob selection one segment to the left.
    pub fn select_previous(&mut self) {
        self.selection = match self.selection {
            Control::Left => Control::Right,
            Control::Press => Control::Left,
            Control::Right => Control::Press,
        };
    }

    /// Moves the knob selection one segment to the right.
    pub fn select_next(&mut self) {
        self.selection = match self.selection {
            Control::Left => Control::Press,
            Control::Press => Control::Right,
            Control::Right => Control::Left,
        };
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
pub fn draw(ctx: &egui::Context, language: Language, actions: &Actions, state: &mut OverlayState) {
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
                geometry.hit_test(local.x, local.y)
            });

            if ctx.input(|input| input.pointer.primary_clicked()) {
                state.outcome = Some(match state.hovered {
                    Some(segment) => OverlayOutcome::Confirm(control_of(segment)),
                    None => OverlayOutcome::Close,
                });
            }

            let painter = ui.painter();
            let border = Stroke::new(1.5_f32, Color32::from_white_alpha(46));
            let half_pi = std::f32::consts::FRAC_PI_2;

            paint_segment(
                painter,
                center,
                &geometry,
                half_pi,
                3.0 * half_pi,
                segment_color(state, WheelSegment::Left),
                border,
            );
            paint_segment(
                painter,
                center,
                &geometry,
                -half_pi,
                half_pi,
                segment_color(state, WheelSegment::Right),
                border,
            );

            let press_color = segment_color(state, WheelSegment::Press);
            painter.circle_filled(center, geometry.inner_radius, press_color);
            painter.circle_stroke(center, geometry.inner_radius, border);

            let label_font = FontId::proportional(13.0);
            let center_font = FontId::proportional(12.0);
            let slot = (geometry.inner_radius + geometry.outer_radius) / 2.0;

            label(
                painter,
                center - egui::vec2(slot, 0.0),
                actions.left.describe(language),
                label_font.clone(),
            );
            label(
                painter,
                center + egui::vec2(slot, 0.0),
                actions.right.describe(language),
                label_font,
            );
            label(
                painter,
                center,
                actions.press.describe(language),
                center_font,
            );

            painter.text(
                rect.center_bottom() - egui::vec2(0.0, 18.0),
                Align2::CENTER_BOTTOM,
                tr(language, TextKey::OverlayHint),
                FontId::proportional(11.0),
                Color32::from_white_alpha(170),
            );
        });
}

fn control_of(segment: WheelSegment) -> Control {
    match segment {
        WheelSegment::Left => Control::Left,
        WheelSegment::Right => Control::Right,
        WheelSegment::Press => Control::Press,
    }
}

fn segment_color(state: &OverlayState, segment: WheelSegment) -> Color32 {
    let selected = control_of(segment) == state.selection;
    let hovered = state.hovered == Some(segment);
    if selected || hovered {
        Color32::from_rgba_unmultiplied(75, 139, 255, 240)
    } else {
        Color32::from_rgba_unmultiplied(22, 26, 36, 235)
    }
}

fn paint_segment(
    painter: &egui::Painter,
    center: Pos2,
    geometry: &WheelGeometry,
    start: f32,
    end: f32,
    fill: Color32,
    stroke: Stroke,
) {
    const STEPS: usize = 28;
    let mut points = Vec::with_capacity((STEPS + 1) * 2);
    for index in 0..=STEPS {
        let angle = start + (end - start) * index as f32 / STEPS as f32;
        points.push(center + egui::vec2(angle.cos(), angle.sin()) * geometry.outer_radius);
    }
    for index in (0..=STEPS).rev() {
        let angle = start + (end - start) * index as f32 / STEPS as f32;
        points.push(center + egui::vec2(angle.cos(), angle.sin()) * geometry.inner_radius);
    }
    painter.add(egui::Shape::convex_polygon(points, fill, stroke));
}

fn label(painter: &egui::Painter, position: Pos2, text: String, font: FontId) {
    let text = truncate(&text, 26);
    painter.text(position, Align2::CENTER_CENTER, text, font, Color32::WHITE);
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
    fn selection_cycles_through_the_three_segments() {
        let mut state = OverlayState::default();
        assert_eq!(state.selection, Control::Press);

        state.select_next();
        assert_eq!(state.selection, Control::Right);
        state.select_next();
        assert_eq!(state.selection, Control::Left);
        state.select_next();
        assert_eq!(state.selection, Control::Press);

        state.select_previous();
        assert_eq!(state.selection, Control::Left);
        state.select_previous();
        assert_eq!(state.selection, Control::Right);
        state.select_previous();
        assert_eq!(state.selection, Control::Press);
    }

    #[test]
    fn segments_map_to_controls() {
        assert_eq!(control_of(WheelSegment::Left), Control::Left);
        assert_eq!(control_of(WheelSegment::Right), Control::Right);
        assert_eq!(control_of(WheelSegment::Press), Control::Press);
    }

    #[test]
    fn long_labels_are_truncated() {
        assert_eq!(truncate("short", 10), "short");
        let truncated = truncate("a very long action label indeed", 10);
        assert_eq!(truncated.chars().count(), 10);
        assert!(truncated.ends_with('…'));
    }
}
