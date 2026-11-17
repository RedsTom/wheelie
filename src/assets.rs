//! Embedded application assets.
//!
//! The logo lives in `assets/logo` and is the single source of truth for the
//! window icon, tray icon and in-app brand mark.

use eframe::egui;
use image::RgbaImage;

const LOGO_4X: &[u8] = include_bytes!("../assets/logo/logo-4x.png");
const LOGO_2X: &[u8] = include_bytes!("../assets/logo/logo-2x.png");

fn decode(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory(bytes)
        .expect("embedded logo PNG must decode")
        .into_rgba8()
}

/// Window and taskbar icon built from the 2x logo.
pub fn window_icon() -> egui::IconData {
    let image = decode(LOGO_2X);
    let width = image.width();
    let height = image.height();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

/// Raw RGBA bytes and size for the system tray icon.
pub fn logo_rgba() -> (Vec<u8>, u32, u32) {
    let image = decode(LOGO_2X);
    let width = image.width();
    let height = image.height();
    (image.into_raw(), width, height)
}

/// Loads the 4x logo as an egui texture for in-app branding.
pub fn load_logo_texture(context: &egui::Context) -> egui::TextureHandle {
    let image = decode(LOGO_4X);
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    context.load_texture("wheelie-logo", color, egui::TextureOptions::LINEAR)
}
