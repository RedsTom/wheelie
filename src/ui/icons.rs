//! Vector icons for action sets.
//!
//! Icons are painted from primitives so they render identically without font
//! fallbacks. The same painter is used in the wheel and in the icon picker.

use crate::core::SetIcon;
use crate::ui::theme;
use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

/// Paints `icon` centered in `rect` with the given color.
pub fn paint(painter: &egui::Painter, rect: Rect, icon: SetIcon, color: Color32) {
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let s = size;
    let stroke = Stroke::new((size * 0.08).clamp(1.0, 3.0), color);

    match icon {
        SetIcon::Volume => {
            let points = vec![
                center + Vec2::new(-0.30 * s, -0.14 * s),
                center + Vec2::new(-0.14 * s, -0.14 * s),
                center + Vec2::new(0.02 * s, -0.30 * s),
                center + Vec2::new(0.02 * s, 0.30 * s),
                center + Vec2::new(-0.14 * s, 0.14 * s),
                center + Vec2::new(-0.30 * s, 0.14 * s),
            ];
            painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
            arc(painter, center, 0.22 * s, -0.9, 0.9, stroke);
            arc(painter, center, 0.34 * s, -0.9, 0.9, stroke);
        }
        SetIcon::Brightness => {
            painter.circle_stroke(center, 0.16 * s, stroke);
            for index in 0..8 {
                let angle = std::f32::consts::TAU * index as f32 / 8.0;
                let from = center + Vec2::angled(angle) * 0.26 * s;
                let to = center + Vec2::angled(angle) * 0.38 * s;
                painter.line_segment([from, to], stroke);
            }
        }
        SetIcon::Scroll => {
            chevron(painter, center + Vec2::new(0.0, -0.16 * s), -1.0, s, stroke);
            chevron(painter, center + Vec2::new(0.0, 0.16 * s), 1.0, s, stroke);
        }
        SetIcon::Keyboard => {
            let body = Rect::from_center_size(center, Vec2::new(0.66 * s, 0.40 * s));
            painter.rect_stroke(body, 0.06 * s, stroke, StrokeKind::Middle);
            for column in -1..=1 {
                let dot = center + Vec2::new(column as f32 * 0.16 * s, -0.06 * s);
                painter.circle_filled(dot, 0.035 * s, color);
            }
            painter.line_segment(
                [
                    center + Vec2::new(-0.16 * s, 0.08 * s),
                    center + Vec2::new(0.16 * s, 0.08 * s),
                ],
                stroke,
            );
        }
        SetIcon::Mouse => {
            let body = Rect::from_center_size(center, Vec2::new(0.32 * s, 0.52 * s));
            painter.rect_stroke(body, 0.16 * s, stroke, StrokeKind::Middle);
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -0.26 * s),
                    center + Vec2::new(0.0, -0.06 * s),
                ],
                stroke,
            );
        }
        SetIcon::Media => {
            painter.circle_filled(center + Vec2::new(-0.10 * s, 0.16 * s), 0.09 * s, color);
            painter.line_segment(
                [
                    center + Vec2::new(-0.01 * s, 0.16 * s),
                    center + Vec2::new(-0.01 * s, -0.24 * s),
                ],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    center + Vec2::new(-0.01 * s, -0.24 * s),
                    center + Vec2::new(0.14 * s, -0.30 * s),
                    center + Vec2::new(0.14 * s, -0.14 * s),
                ],
                stroke,
            ));
        }
        SetIcon::Star => {
            let mut points = Vec::with_capacity(10);
            for index in 0..10 {
                let angle =
                    -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * index as f32 / 10.0;
                let radius = if index % 2 == 0 { 0.36 * s } else { 0.15 * s };
                points.push(center + Vec2::angled(angle) * radius);
            }
            painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
        }
        SetIcon::Heart => {
            painter.circle_filled(center + Vec2::new(-0.11 * s, -0.08 * s), 0.14 * s, color);
            painter.circle_filled(center + Vec2::new(0.11 * s, -0.08 * s), 0.14 * s, color);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + Vec2::new(-0.24 * s, -0.02 * s),
                    center + Vec2::new(0.24 * s, -0.02 * s),
                    center + Vec2::new(0.0, 0.30 * s),
                ],
                color,
                Stroke::NONE,
            ));
        }
        SetIcon::Gear => {
            painter.circle_stroke(center, 0.15 * s, stroke);
            for index in 0..8 {
                let angle = std::f32::consts::TAU * index as f32 / 8.0;
                let from = center + Vec2::angled(angle) * 0.22 * s;
                let to = center + Vec2::angled(angle) * 0.32 * s;
                painter.line_segment([from, to], stroke);
            }
            painter.circle_filled(center, 0.05 * s, color);
        }
        SetIcon::Search => {
            painter.circle_stroke(center + Vec2::new(-0.05 * s, -0.05 * s), 0.18 * s, stroke);
            painter.line_segment(
                [
                    center + Vec2::new(0.09 * s, 0.09 * s),
                    center + Vec2::new(0.27 * s, 0.27 * s),
                ],
                stroke,
            );
        }
        SetIcon::Folder => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + Vec2::new(-0.30 * s, -0.16 * s),
                    center + Vec2::new(-0.06 * s, -0.16 * s),
                    center + Vec2::new(0.02 * s, -0.06 * s),
                    center + Vec2::new(0.30 * s, -0.06 * s),
                    center + Vec2::new(0.30 * s, 0.22 * s),
                    center + Vec2::new(-0.30 * s, 0.22 * s),
                ],
                color,
                Stroke::NONE,
            ));
        }
        SetIcon::App => {
            let cell = 0.20 * s;
            let gap = 0.05 * s;
            for row in 0..2 {
                for column in 0..2 {
                    let offset = Vec2::new(
                        (column as f32 - 0.5) * (cell + gap),
                        (row as f32 - 0.5) * (cell + gap),
                    );
                    painter.rect_filled(
                        Rect::from_center_size(center + offset, Vec2::splat(cell)),
                        0.03 * s,
                        color,
                    );
                }
            }
        }
    }
}

/// Grid of selectable icon buttons. Returns true when the selection changed.
pub fn picker(ui: &mut egui::Ui, selected: &mut SetIcon) -> bool {
    let mut changed = false;

    for row in SetIcon::ALL.chunks(6) {
        ui.horizontal(|ui| {
            for icon in row {
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
                let painter = ui.painter().clone();
                let active = *selected == *icon;

                if active {
                    painter.rect_filled(rect, 6, theme::accent_soft());
                    painter.rect_stroke(
                        rect,
                        6,
                        Stroke::new(1.0_f32, theme::accent(ui)),
                        StrokeKind::Inside,
                    );
                } else if response.hovered() {
                    painter.rect_filled(rect, 6, ui.visuals().widgets.hovered.weak_bg_fill);
                }

                let color = if active {
                    theme::accent(ui)
                } else {
                    ui.visuals().widgets.inactive.fg_stroke.color
                };
                paint(&painter, rect.shrink(7.0), *icon, color);

                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    *selected = *icon;
                    changed = true;
                }
            }
        });
    }

    changed
}

fn arc(painter: &egui::Painter, center: Pos2, radius: f32, start: f32, end: f32, stroke: Stroke) {
    const STEPS: usize = 12;
    let points: Vec<Pos2> = (0..=STEPS)
        .map(|index| {
            let angle = start + (end - start) * index as f32 / STEPS as f32;
            center + Vec2::angled(angle) * radius
        })
        .collect();
    painter.add(egui::Shape::line(points, stroke));
}

fn chevron(painter: &egui::Painter, center: Pos2, direction: f32, size: f32, stroke: Stroke) {
    painter.add(egui::Shape::line(
        vec![
            center + Vec2::new(-0.18 * size, 0.08 * size * direction),
            center + Vec2::new(0.0, -0.08 * size * direction),
            center + Vec2::new(0.18 * size, 0.08 * size * direction),
        ],
        stroke,
    ));
}
