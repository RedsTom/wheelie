//! Shared visual components.

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Vec2};

use crate::ui::theme;

/// Card container used by settings pages.
pub fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    theme::card_frame(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add_contents(ui);
    });
}

/// Compact page title and muted subtitle.
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .font(theme::heading_font(24.0))
            .color(theme::card_text(ui)),
    );
    ui.add_space(3.0);
    ui.label(RichText::new(subtitle).size(13.0).color(theme::muted(ui)));
    ui.add_space(14.0);
}

/// Section title used inside a card.
pub fn section_title(ui: &mut egui::Ui, title: &str, subtitle: Option<&str>) {
    ui.label(
        RichText::new(title)
            .font(theme::heading_font(16.0))
            .color(theme::card_text(ui)),
    );
    if let Some(subtitle) = subtitle {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(12.5).color(theme::muted(ui)));
    }
}

/// Filled primary action. Height matches status pills.
pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(14.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(theme::ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
        .min_size(Vec2::new(0.0, theme::CONTROL_HEIGHT)),
    )
}

/// Subtle outlined action. Height matches status pills.
pub fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(13.5).color(theme::card_text(ui)))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0_f32, theme::border(ui)))
            .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
            .min_size(Vec2::new(0.0, theme::CONTROL_HEIGHT)),
    )
}

/// Compact sidebar navigation item.
pub fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let height = theme::CONTROL_HEIGHT + 4.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
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
            rect.left_top() + Vec2::new(0.0, 8.0),
            Vec2::new(3.0, rect.height() - 16.0),
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
        theme::heading_font(14.0)
    } else {
        FontId::proportional(14.0)
    };

    ui.painter().text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        font,
        text_color,
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Status pill with exactly the same control height as buttons.
pub fn status_pill(ui: &mut egui::Ui, label: &str, color: Color32) {
    let _ = ui.add_enabled(
        false,
        egui::Button::new(RichText::new(label).size(14.0).strong().color(color))
            .fill(color.gamma_multiply(0.16))
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
            .min_size(Vec2::new(0.0, theme::CONTROL_HEIGHT)),
    );
}

/// Centered empty state for a section that is not implemented yet.
pub fn empty_state(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    badge: &str,
    badge_color: Color32,
    logo: &egui::TextureHandle,
) {
    ui.add_space(18.0);
    ui.vertical_centered(|ui| {
        logo_image(ui, logo, 76.0);
        ui.add_space(12.0);
        ui.label(
            RichText::new(title)
                .font(theme::heading_font(18.0))
                .color(theme::card_text(ui)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(description)
                .size(13.0)
                .color(theme::muted(ui)),
        );
        ui.add_space(12.0);
        status_pill(ui, badge, badge_color);
    });
    ui.add_space(18.0);
}

/// Displays the embedded logo at a fixed square size.
pub fn logo_image(ui: &mut egui::Ui, texture: &egui::TextureHandle, size: f32) {
    ui.add(egui::Image::new(texture).fit_to_exact_size(Vec2::splat(size)));
}

/// Numbered roadmap row.
pub fn timeline_item(ui: &mut egui::Ui, index: usize, title: &str, body: &str, active: bool) {
    let accent = theme::accent(ui);
    let muted = theme::muted(ui);
    let border = theme::border(ui);
    let circle = if active { accent } else { border };

    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 12.0, circle.gamma_multiply(0.16));
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            index.to_string(),
            theme::heading_font(12.0),
            circle,
        );

        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::heading_font(14.0))
                    .color(theme::card_text(ui)),
            );
            ui.label(RichText::new(body).size(12.5).color(muted));
        });
    });
}

/// Key and modifier picker. Returns true when the chord changed.
pub fn chord_picker(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    language: crate::core::Language,
    chord: &mut crate::core::KeyChord,
) -> bool {
    use crate::core::{KeyCode, TextKey, tr};

    let mut changed = false;

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(chord.key.as_str())
            .width(92.0)
            .show_ui(ui, |ui| {
                for key in KeyCode::ALL {
                    if ui
                        .selectable_value(&mut chord.key, *key, key.as_str())
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

        changed |= ui
            .checkbox(
                &mut chord.modifiers.control,
                tr(language, TextKey::ModifierControl),
            )
            .changed();
        changed |= ui
            .checkbox(&mut chord.modifiers.alt, tr(language, TextKey::ModifierAlt))
            .changed();
        changed |= ui
            .checkbox(
                &mut chord.modifiers.shift,
                tr(language, TextKey::ModifierShift),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut chord.modifiers.meta,
                tr(language, TextKey::ModifierMeta),
            )
            .changed();
    });

    changed
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
                    .font(theme::heading_font(14.0))
                    .color(theme::card_text(ui)),
            );
            ui.label(RichText::new(description).size(12.5).color(muted));
        });
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            add_control,
        );
    });
}
