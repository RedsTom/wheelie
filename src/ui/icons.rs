//! Phosphor icons for action sets.
//!
//! The icon font is registered by [`crate::ui::theme::install_fonts`]; this
//! module only maps the semantic icon of a set to its glyph and draws it.

use crate::core::SetIcon;
use crate::ui::theme;
use eframe::egui;
use egui::{Align2, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind, Vec2};
use egui_phosphor::regular as ph;

/// Glyph used for an icon.
pub fn glyph(icon: SetIcon) -> &'static str {
    match icon {
        SetIcon::Volume => ph::SPEAKER_HIGH,
        SetIcon::Brightness => ph::SUN,
        SetIcon::Scroll => ph::ARROWS_VERTICAL,
        SetIcon::ScrollHorizontal => ph::ARROWS_HORIZONTAL,
        SetIcon::Keyboard => ph::KEYBOARD,
        SetIcon::Mouse => ph::MOUSE,
        SetIcon::Media => ph::MUSIC_NOTES,
        SetIcon::Star => ph::STAR,
        SetIcon::Heart => ph::HEART,
        SetIcon::Gear => ph::GEAR,
        SetIcon::Search => ph::MAGNIFYING_GLASS,
        SetIcon::Folder => ph::FOLDER,
        SetIcon::App => ph::ROCKET_LAUNCH,
    }
}

/// Paints `icon` centered in `rect` with the given color.
pub fn paint(painter: &egui::Painter, rect: Rect, icon: SetIcon, color: egui::Color32) {
    let size = rect.width().min(rect.height());
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph(icon),
        FontId::new(size * 0.95, FontFamily::Proportional),
        color,
    );
}

/// Grid of selectable icon buttons. Returns true when the selection changed.
pub fn picker(ui: &mut egui::Ui, selected: &mut SetIcon) -> bool {
    let mut changed = false;

    for row in SetIcon::ALL.chunks(7) {
        ui.horizontal(|ui| {
            for icon in row {
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                let painter = ui.painter().clone();
                let active = *selected == *icon;

                if active {
                    painter.rect_filled(rect, 7, theme::accent_soft());
                    painter.rect_stroke(
                        rect,
                        7,
                        Stroke::new(1.0_f32, theme::accent(ui)),
                        StrokeKind::Inside,
                    );
                } else if response.hovered() {
                    painter.rect_filled(rect, 7, ui.visuals().widgets.hovered.weak_bg_fill);
                }

                let color = if active {
                    theme::accent(ui)
                } else {
                    ui.visuals().widgets.inactive.fg_stroke.color
                };
                paint(&painter, rect.shrink(6.0), *icon, color);

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
