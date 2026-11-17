//! Visual theme for Wheelie.
//!
//! The palette is intentionally simple: deep slate surfaces, a single blue
//! accent, generous spacing and large rounded cards. It should feel close to a
//! Fluent settings app without imitating Windows chrome.

use eframe::egui;
use egui::epaint::Shadow;
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Visuals,
};
use std::sync::Arc;

/// Primary accent used by actions, selection and the wheel.
pub const ACCENT: Color32 = Color32::from_rgb(75, 139, 255);

/// Success color for connected/running states.
pub const SUCCESS: Color32 = Color32::from_rgb(52, 211, 153);

/// Warning color for paused or limited states.
pub const WARNING: Color32 = Color32::from_rgb(245, 158, 11);

/// Danger color for errors.
pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113);

/// Outer card radius.
pub const RADIUS_CARD: u8 = 8;

/// Smaller control radius.
pub const RADIUS_CONTROL: u8 = 4;

/// Height shared by buttons, pills and combo boxes.
pub const CONTROL_HEIGHT: f32 = 32.0;

const DARK_BG: Color32 = Color32::from_rgb(13, 16, 23);
const DARK_SIDEBAR: Color32 = Color32::from_rgb(17, 20, 28);
const DARK_CARD: Color32 = Color32::from_rgb(24, 28, 38);
const DARK_CARD_HOVER: Color32 = Color32::from_rgb(31, 36, 48);
const DARK_BORDER: Color32 = Color32::from_rgb(44, 51, 64);
const DARK_TEXT: Color32 = Color32::from_rgb(235, 239, 247);
const DARK_MUTED: Color32 = Color32::from_rgb(148, 160, 181);

const LIGHT_BG: Color32 = Color32::from_rgb(247, 248, 250);
const LIGHT_SIDEBAR: Color32 = Color32::from_rgb(240, 242, 245);
const LIGHT_CARD: Color32 = Color32::from_rgb(255, 255, 255);
const LIGHT_CARD_HOVER: Color32 = Color32::from_rgb(248, 249, 251);
const LIGHT_BORDER: Color32 = Color32::from_rgb(225, 228, 233);
const LIGHT_TEXT: Color32 = Color32::from_rgb(23, 26, 32);
const LIGHT_MUTED: Color32 = Color32::from_rgb(110, 118, 129);

pub use crate::core::ThemeMode;

/// Installs the bundled Inter family. Call once, before the first frame.
pub fn install_fonts(context: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "inter-regular".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "inter-semibold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter-regular".to_owned());
    fonts.families.insert(
        FontFamily::Name("InterSemiBold".into()),
        vec!["inter-semibold".to_owned(), "inter-regular".to_owned()],
    );

    context.set_fonts(fonts);
}

/// Semibold font used for headings and selected navigation labels.
pub fn heading_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("InterSemiBold".into()))
}

/// Applies the selected theme to the egui context.
pub fn apply(context: &egui::Context, mode: ThemeMode) {
    context.send_viewport_cmd(egui::ViewportCommand::SetTheme(match mode {
        ThemeMode::Dark => egui::SystemTheme::Dark,
        ThemeMode::Light => egui::SystemTheme::Light,
    }));

    let dark = matches!(mode, ThemeMode::Dark);
    let (bg, card, card_hover, border, text) = if dark {
        (DARK_BG, DARK_CARD, DARK_CARD_HOVER, DARK_BORDER, DARK_TEXT)
    } else {
        (
            LIGHT_BG,
            LIGHT_CARD,
            LIGHT_CARD_HOVER,
            LIGHT_BORDER,
            LIGHT_TEXT,
        )
    };

    let mut style = (*context.style()).clone();
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.dark_mode = dark;
    visuals.panel_fill = bg;
    visuals.window_fill = card;
    visuals.faint_bg_color = card;
    visuals.extreme_bg_color = bg;
    visuals.code_bg_color = card;
    visuals.window_stroke = Stroke::new(1.0_f32, border);
    visuals.window_corner_radius = CornerRadius::same(RADIUS_CARD);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS_CONTROL);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.22);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.window_shadow = Shadow {
        offset: [0, 12],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 90 } else { 28 }),
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 80 } else { 24 }),
    };

    visuals.widgets.noninteractive.bg_fill = card;
    visuals.widgets.noninteractive.weak_bg_fill = card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(RADIUS_CONTROL);

    visuals.widgets.inactive.bg_fill = card;
    visuals.widgets.inactive.weak_bg_fill = card;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(RADIUS_CONTROL);

    visuals.widgets.hovered.bg_fill = card_hover;
    visuals.widgets.hovered.weak_bg_fill = card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.65));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(RADIUS_CONTROL);

    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.18);
    visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.18);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.active.corner_radius = CornerRadius::same(RADIUS_CONTROL);

    visuals.widgets.open.bg_fill = card_hover;
    visuals.widgets.open.weak_bg_fill = card_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.65));
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.open.corner_radius = CornerRadius::same(RADIUS_CONTROL);

    visuals.override_text_color = None;
    visuals.slider_trailing_fill = true;

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);
    style.spacing.interact_size = egui::vec2(40.0, CONTROL_HEIGHT);
    style.spacing.menu_margin = Margin::same(6);
    style.animation_time = 0.08;
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Name("InterSemiBold".into())),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );

    context.set_style(style);
}

/// Sidebar background for the selected theme.
pub fn sidebar_fill(mode: ThemeMode) -> Color32 {
    match mode {
        ThemeMode::Dark => DARK_SIDEBAR,
        ThemeMode::Light => LIGHT_SIDEBAR,
    }
}

/// Muted text color for the selected theme.
pub fn muted_text(mode: ThemeMode) -> Color32 {
    match mode {
        ThemeMode::Dark => DARK_MUTED,
        ThemeMode::Light => LIGHT_MUTED,
    }
}

/// Background color for the selected theme.
pub fn background(mode: ThemeMode) -> Color32 {
    match mode {
        ThemeMode::Dark => DARK_BG,
        ThemeMode::Light => LIGHT_BG,
    }
}

/// Card frame with a border and compact padding.
pub fn card_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::NONE
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(
            1.0_f32,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(CornerRadius::same(RADIUS_CARD))
        .inner_margin(Margin::same(14))
}

/// Soft accent background used by selected navigation items.
pub fn accent_soft() -> Color32 {
    ACCENT.gamma_multiply(0.14)
}

/// Palette used for text on cards.
pub fn card_text(ui: &egui::Ui) -> Color32 {
    ui.visuals().widgets.noninteractive.fg_stroke.color
}

/// Muted text derived from the current visuals.
pub fn muted(ui: &egui::Ui) -> Color32 {
    ui.visuals().weak_text_color()
}

/// Accent color derived from the current visuals.
pub fn accent(ui: &egui::Ui) -> Color32 {
    ui.visuals().hyperlink_color
}

/// Border color derived from the current visuals.
pub fn border(ui: &egui::Ui) -> Color32 {
    ui.visuals().widgets.noninteractive.bg_stroke.color
}
