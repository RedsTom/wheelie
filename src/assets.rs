//! Embedded application assets shared outside the UI layer.
//!
//! The logo lives in `assets/logo` and is the single source of truth for the
//! window icon, tray icon and in-app brand mark. The gpui shell loads it
//! through [`crate::ui::assets`]; this module only exposes the raw pixels the
//! system tray needs.

use image::RgbaImage;

const LOGO_2X: &[u8] = include_bytes!("../assets/logo/logo-2x.png");

fn decode(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory(bytes)
        .expect("embedded logo PNG must decode")
        .into_rgba8()
}

/// Raw RGBA bytes and size for the system tray icon.
pub fn logo_rgba() -> (Vec<u8>, u32, u32) {
    let image = decode(LOGO_2X);
    let width = image.width();
    let height = image.height();
    (image.into_raw(), width, height)
}
