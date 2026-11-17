//! Shared visual components.

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, RichText, Sense, Stroke, Vec2};

use crate::ui::theme;

/// Card container used by settings pages.
pub fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::card_frame(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add_contents(ui);
    });
}

/// Large page title and muted subtitle.
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .font(theme::heading_font(32.0))
            .color(theme::card_text(ui)),
    );
    ui.add_space(6.0);
    ui.label(RichText::new(subtitle).size(14.5).color(theme::muted(ui)));
    ui.add_space(24.0);
}

/// Section title used inside a card.
pub fn section_title(ui: &mut egui::Ui, title: &str, subtitle: Option<&str>) {
    ui.label(
        RichText::new(title)
            .font(theme::heading_font(18.0))
            .color(theme::card_text(ui)),
    );
    if let Some(subtitle) = subtitle {
        ui.add_space(3.0);
        ui.label(RichText::new(subtitle).size(13.0).color(theme::muted(ui)));
    }
}

/// Filled primary action.
pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(15.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(theme::ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
        .min_size(Vec2::new(156.0, 42.0)),
    )
}

/// Subtle outlined action.
pub fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(14.5).color(theme::card_text(ui)))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0_f32, theme::border(ui)))
            .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
            .min_size(Vec2::new(132.0, 40.0)),
    )
}

/// Large sidebar navigation item.
pub fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 44.0), Sense::click());
    let visuals = ui.visuals();

    let background = if selected {
        theme::accent_soft()
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };

    if background != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_CONTROL), background);
    }

    if selected {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + Vec2::new(0.0, 11.0),
            Vec2::new(3.0, rect.height() - 22.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), theme::accent(ui));
    }

    let text_color = if selected {
        visuals.widgets.noninteractive.fg_stroke.color
    } else {
        visuals.widgets.inactive.fg_stroke.color
    };
    let font = if selected {
        theme::heading_font(15.0)
    } else {
        FontId::proportional(15.0)
    };

    ui.painter().text(
        rect.left_center() + Vec2::new(16.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        font,
        text_color,
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Small status indicator with a colored background.
pub fn status_pill(ui: &mut egui::Ui, label: &str, color: Color32) {
    let _ = egui::Frame::NONE
        .fill(color.gamma_multiply(0.16))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 7))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(12.5).strong().color(color));
        });
}

/// Centered empty state for a section that is not implemented yet.
pub fn empty_state(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    badge: &str,
    badge_color: Color32,
) {
    ui.add_space(28.0);
    ui.vertical_centered(|ui| {
        wheel_logo(ui, 96.0, theme::accent(ui));
        ui.add_space(18.0);
        ui.label(
            RichText::new(title)
                .font(theme::heading_font(24.0))
                .color(theme::card_text(ui)),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(description)
                .size(14.0)
                .color(theme::muted(ui)),
        );
        ui.add_space(16.0);
        status_pill(ui, badge, badge_color);
    });
    ui.add_space(28.0);
}

/// Draws a simple wheel mark without external icon fonts.
pub fn wheel_logo(ui: &mut egui::Ui, diameter: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    let radius = diameter * 0.40;

    painter.circle_stroke(center, radius, Stroke::new(diameter * 0.045, color));
    painter.circle_filled(center, diameter * 0.075, color);

    for index in 0..4 {
        let angle = index as f32 * std::f32::consts::TAU / 4.0;
        let direction = Vec2::new(angle.cos(), angle.sin());
        painter.line_segment(
            [
                center + direction * diameter * 0.14,
                center + direction * diameter * 0.32,
            ],
            Stroke::new(diameter * 0.032, color),
        );
    }
}

/// Numbered roadmap row.
pub fn timeline_item(ui: &mut egui::Ui, index: usize, title: &str, body: &str, active: bool) {
    let accent = theme::accent(ui);
    let muted = theme::muted(ui);
    let border = theme::border(ui);
    let circle = if active { accent } else { border };

    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 15.0, circle.gamma_multiply(0.16));
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            index.to_string(),
            theme::heading_font(13.0),
            circle,
        );

        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::heading_font(15.0))
                    .color(theme::card_text(ui)),
            );
            ui.label(RichText::new(body).size(13.0).color(muted));
        });
    });
}

/// Label/description row with a control aligned to the right.
pub fn setting_row(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    add_control: impl FnOnce(&mut egui::Ui),
) {
    let muted = theme::muted(ui);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::heading_font(15.0))
                    .color(theme::card_text(ui)),
            );
            ui.label(RichText::new(description).size(13.0).color(muted));
        });
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            add_control,
        );
    });
}
