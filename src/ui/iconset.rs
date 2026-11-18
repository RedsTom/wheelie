//! Lucide icon lookup for the gpui UI.
//!
//! The eframe pages still draw Phosphor glyphs; this module maps the semantic
//! [`SetIcon`] of an action set to its Lucide equivalent for the gpui port.
//! Names resolve through `LucideIcon::from_name`, so upstream aliases keep
//! working; the test below guards the whole table.

use crate::core::SetIcon;
use gpui::{Rgba, Styled};
use gpui_icons::LucideIcon;

/// Lucide canonical (or alias) name for a set icon.
pub fn name(icon: SetIcon) -> &'static str {
    match icon {
        SetIcon::Volume => "volume-2",
        SetIcon::Brightness => "sun",
        SetIcon::Scroll => "arrow-up-down",
        SetIcon::ScrollHorizontal => "arrow-left-right",
        SetIcon::Keyboard => "keyboard",
        SetIcon::Mouse => "mouse",
        SetIcon::Media => "music",
        SetIcon::Star => "star",
        SetIcon::Heart => "heart",
        SetIcon::Gear => "settings",
        SetIcon::Search => "search",
        SetIcon::Folder => "folder",
        SetIcon::App => "rocket",
        SetIcon::Video => "film",
        SetIcon::Browser => "globe",
        SetIcon::Mail => "mail",
        SetIcon::Calculator => "calculator",
        SetIcon::Terminal => "terminal",
        SetIcon::Photo => "image",
        SetIcon::Game => "gamepad-2",
        SetIcon::Home => "house",
        SetIcon::Lock => "lock",
        SetIcon::Wifi => "wifi",
        SetIcon::Bluetooth => "bluetooth",
        SetIcon::Battery => "battery-full",
        SetIcon::Clock => "clock",
        SetIcon::Calendar => "calendar",
        SetIcon::Bell => "bell",
        SetIcon::Bookmark => "bookmark",
        SetIcon::Pin => "pin",
        SetIcon::Code => "code",
        SetIcon::Database => "database",
        SetIcon::Cloud => "cloud",
        SetIcon::Download => "download",
        SetIcon::Upload => "upload",
        SetIcon::Monitor => "monitor",
        SetIcon::Laptop => "laptop",
        SetIcon::Phone => "smartphone",
        SetIcon::Headphones => "headphones",
        SetIcon::Microphone => "mic",
        SetIcon::Palette => "palette",
        SetIcon::Pencil => "pencil",
        SetIcon::Compass => "compass",
        SetIcon::Flag => "flag",
        SetIcon::Gift => "gift",
        SetIcon::Lightning => "zap",
        SetIcon::Fire => "flame",
        SetIcon::Leaf => "leaf",
        SetIcon::Moon => "moon",
        SetIcon::Prohibit => "ban",
    }
}

/// Resolves a set icon to its Lucide constant.
///
/// Panics only for a name the test does not already cover.
pub fn lucide_icon(icon: SetIcon) -> LucideIcon {
    LucideIcon::from_name(name(icon)).expect("SetIcon maps to a known Lucide icon")
}

/// Renders a set icon at `size` pixels in `color`.
pub fn element(icon: SetIcon, size: f32, color: Rgba) -> impl gpui::IntoElement {
    gpui_icons::lucide(lucide_icon(icon))
        .size(gpui::px(size))
        .text_color(color)
}

/// Renders an arbitrary Lucide icon (inline UI glyphs).
pub fn lucide_element(icon: LucideIcon, size: f32, color: Rgba) -> impl gpui::IntoElement {
    gpui_icons::lucide(icon)
        .size(gpui::px(size))
        .text_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_set_icon_resolves_to_lucide() {
        let missing: Vec<_> = SetIcon::ALL
            .iter()
            .filter(|icon| LucideIcon::from_name(name(**icon)).is_none())
            .map(|icon| name(*icon))
            .collect();
        assert!(missing.is_empty(), "unknown Lucide icon names: {missing:?}");
    }
}
