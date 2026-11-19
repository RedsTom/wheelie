//! Visual tokens for the gpui settings shell.
//!
//! Same palette as the eframe theme (deep slate surfaces, single blue accent)
//! exposed as plain color values gpui can consume directly. The eframe theme
//! in `theme.rs` keeps serving the legacy pages until they are ported.

use crate::core::ThemeMode;
use gpui::Rgba;

/// Builds an opaque color from a `0xRRGGBB` literal (const-compatible).
pub(crate) const fn rgb(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

/// Builds a color from a `0xRRGGBBAA` literal (const-compatible).
pub(crate) const fn rgba(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 24) & 0xFF) as f32 / 255.0,
        g: ((hex >> 16) & 0xFF) as f32 / 255.0,
        b: ((hex >> 8) & 0xFF) as f32 / 255.0,
        a: (hex & 0xFF) as f32 / 255.0,
    }
}

/// Primary accent used by actions, selection and the wheel.
pub const ACCENT: Rgba = rgb(0x4B8BFF);

/// Success color for connected/running states.
pub const SUCCESS: Rgba = rgb(0x34D399);

/// Warning color for paused or limited states.
pub const WARNING: Rgba = rgb(0xF59E0B);

/// Danger color for errors.
pub const DANGER: Rgba = rgb(0xF87171);

/// Fully transparent color, used for hidden-but-reserved controls.
pub const TRANSPARENT: Rgba = rgba(0x00000000);

/// Outer card radius.
pub const RADIUS_CARD: f32 = 8.0;

/// Smaller control radius.
pub const RADIUS_CONTROL: f32 = 4.0;

/// Height shared by buttons, pills and combo boxes.
pub const CONTROL_HEIGHT: f32 = 32.0;

/// Sidebar width of the settings shell.
pub const SIDEBAR_WIDTH: f32 = 212.0;

const DARK_BG: Rgba = rgb(0x0D1017);
const DARK_SIDEBAR: Rgba = rgb(0x11141C);
const DARK_CARD: Rgba = rgb(0x181C26);
const DARK_CARD_HOVER: Rgba = rgb(0x1F2430);
const DARK_BORDER: Rgba = rgb(0x2C3340);
const DARK_TEXT: Rgba = rgb(0xEBEFF7);
const DARK_MUTED: Rgba = rgb(0x94A0B5);
const DARK_ACCENT_SOFT: Rgba = rgba(0x4B8BFF24);

const LIGHT_BG: Rgba = rgb(0xF7F8FA);
const LIGHT_SIDEBAR: Rgba = rgb(0xF0F2F5);
const LIGHT_CARD: Rgba = rgb(0xFFFFFF);
const LIGHT_CARD_HOVER: Rgba = rgb(0xF8F9FB);
const LIGHT_BORDER: Rgba = rgb(0xE1E4E9);
const LIGHT_TEXT: Rgba = rgb(0x171A20);
const LIGHT_MUTED: Rgba = rgb(0x6E7681);
const LIGHT_ACCENT_SOFT: Rgba = rgba(0x4B8BFF1F);

/// Color set for one theme mode.
#[derive(Clone, Copy)]
pub struct Tokens {
    /// Page background.
    pub bg: Rgba,
    /// Sidebar background.
    pub sidebar: Rgba,
    /// Card and control background.
    pub card: Rgba,
    /// Hovered card or control background.
    pub card_hover: Rgba,
    /// Hairline border.
    pub border: Rgba,
    /// Primary text.
    pub text: Rgba,
    /// Secondary text.
    pub muted: Rgba,
    /// Soft accent background for selected items.
    pub accent_soft: Rgba,
}

/// Tokens for the selected theme mode.
pub fn tokens(mode: ThemeMode) -> Tokens {
    match mode {
        ThemeMode::Dark => Tokens {
            bg: DARK_BG,
            sidebar: DARK_SIDEBAR,
            card: DARK_CARD,
            card_hover: DARK_CARD_HOVER,
            border: DARK_BORDER,
            text: DARK_TEXT,
            muted: DARK_MUTED,
            accent_soft: DARK_ACCENT_SOFT,
        },
        ThemeMode::Light => Tokens {
            bg: LIGHT_BG,
            sidebar: LIGHT_SIDEBAR,
            card: LIGHT_CARD,
            card_hover: LIGHT_CARD_HOVER,
            border: LIGHT_BORDER,
            text: LIGHT_TEXT,
            muted: LIGHT_MUTED,
            accent_soft: LIGHT_ACCENT_SOFT,
        },
    }
}
