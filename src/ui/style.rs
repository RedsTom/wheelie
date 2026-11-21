//! Visual tokens for the gpui settings shell.
//!
//! Deep slate surfaces and a single blue accent, exposed as plain color values
//! gpui can consume directly. `shell::sync_kit_theme` pushes the same palette
//! into gpui-kit's global theme so kit-backed controls follow the app's
//! dark/light mode; pages read [`tokens`] for their own painting.

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

/// License shown in the About page and the sidebar footer.
pub const APP_LICENSE: &str = "GPL-3.0";

/// Neutral backdrop behind the startup release notice.
pub const MODAL_SCRIM: Rgba = rgba(0x00000073);

/// Tooltip background, opaque so the foreground keeps a known contrast.
pub const TOOLTIP_BG: Rgba = rgb(0x111827);
/// Tooltip foreground.
pub const TOOLTIP_FG: Rgba = rgb(0xFFFFFF);

/// Neutral wash behind the selected wheel name.
pub const WHEEL_TITLE_BG: Rgba = rgba(0x0000008C);

/// Hover/active variants used by gpui-kit buttons.
pub const PRIMARY_HOVER: Rgba = rgb(0x3D79E6);
pub const PRIMARY_ACTIVE: Rgba = rgb(0x2F66CC);
pub const DANGER_HOVER: Rgba = rgb(0xDC2626);
pub const DANGER_ACTIVE: Rgba = rgb(0xB91C1C);
pub const SUCCESS_FOREGROUND: Rgba = rgb(0x062016);
pub const SUCCESS_HOVER: Rgba = rgb(0x10B981);
pub const SUCCESS_ACTIVE: Rgba = rgb(0x059669);
pub const WARNING_FOREGROUND: Rgba = rgb(0x2A1600);
pub const WARNING_HOVER: Rgba = rgb(0xD97706);
pub const WARNING_ACTIVE: Rgba = rgb(0xB45309);

/// Opaque ink used on the accent-colored kit buttons in both themes.
pub const ON_ACCENT: Rgba = rgb(0xFFFFFF);

/// Wheel sector fill when neither selected nor hovered.
pub const WHEEL_BASE: Rgba = rgb(0x181C26);
/// Wheel sector fill under the pointer.
pub const WHEEL_HOVER: Rgba = rgb(0x262C3A);
/// Primary ink on the wheel disc (both themes keep the dark disc).
pub const ON_WHEEL: Rgba = rgb(0xFFFFFF);
/// Secondary ink on the wheel disc.
pub const ON_WHEEL_DIM: Rgba = rgba(0xFFFFFFDC);

/// gpui-kit hover tint in dark mode (kit widgets read the kit theme, not
/// [`tokens`], so this value must be pushed from `shell::sync_kit_theme`).
pub const KIT_HOVER_DARK: Rgba = rgb(0x2C3340);
/// gpui-kit hover tint in light mode.
pub const KIT_HOVER_LIGHT: Rgba = rgb(0xDDE1E7);

/// Kit hover tint for `mode`, kept here so no UI module hard-codes a color.
pub const fn kit_hover(mode: ThemeMode) -> Rgba {
    match mode {
        ThemeMode::Dark => KIT_HOVER_DARK,
        ThemeMode::Light => KIT_HOVER_LIGHT,
    }
}

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
const LIGHT_MUTED: Rgba = rgb(0x5F6873);
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

/// Same RGB as `color` with the requested alpha, for translucent washes.
pub(crate) const fn alpha(color: Rgba, a: f32) -> Rgba {
    Rgba { a, ..color }
}

/// Linear interpolation between two colors, alpha included.
pub(crate) fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Opaque foreground derived from `color` for the surface of `mode`.
///
/// Tinted status pills and tinted icons used to paint the raw status color on a
/// translucent wash of the same color. In light mode that failed WCAG AA (the
/// wash lifts the surface and the ink stays pale). This helper blends `color`
/// toward the mode's primary ink until the result reaches 4.5:1 on the card,
/// which also clears the 3:1 requirement for large glyphs.
pub fn on_surface(color: Rgba, mode: ThemeMode) -> Rgba {
    let t = tokens(mode);
    // A tinted chip can sit on the card or directly on the page. Check both
    // surfaces, otherwise a color that passes on the card fails on the page.
    let readable =
        |candidate: Rgba| contrast(candidate, t.card) >= 4.5 && contrast(candidate, t.bg) >= 4.5;
    let mut candidate = Rgba { a: 1.0, ..color };
    if readable(candidate) {
        return candidate;
    }
    for _ in 1..=20 {
        candidate = mix(candidate, t.text, 0.05);
        if readable(candidate) {
            return candidate;
        }
    }
    t.text
}

/// Relative luminance of an opaque color (WCAG 2.1).
pub(crate) fn luminance(color: Rgba) -> f32 {
    let channel = |value: f32| {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

/// Contrast ratio between two opaque colors (WCAG 2.1), 1.0 to 21.0.
pub(crate) fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (light, dark) = (luminance(a), luminance(b));
    let (high, low) = if light >= dark {
        (light, dark)
    } else {
        (dark, light)
    };
    (high + 0.05) / (low + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ink must stay readable on every surface it is drawn over, in both
    /// themes: a stale or inverted palette is exactly what made the light theme
    /// unusable once.
    #[test]
    fn every_theme_keeps_its_text_readable() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let t = tokens(mode);
            for surface in [t.bg, t.card, t.card_hover, t.sidebar] {
                assert!(
                    contrast(t.text, surface) >= 4.5,
                    "{mode:?}: primary text on {surface:?} is too faint"
                );
                assert!(
                    contrast(t.muted, surface) >= 4.5,
                    "{mode:?}: secondary text on {surface:?} is too faint"
                );
            }
        }
    }

    /// Tinted status ink is opaque and legible in both themes, both as text
    /// (4.5:1) and as a large glyph (3:1).
    #[test]
    fn status_colors_meet_aa_on_every_surface() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let t = tokens(mode);
            for color in [SUCCESS, WARNING, DANGER, ACCENT] {
                let ink = on_surface(color, mode);
                assert_eq!(ink.a, 1.0, "{mode:?}: status ink must be opaque");
                for surface in [t.card, t.bg] {
                    assert!(
                        contrast(ink, surface) >= 4.5,
                        "{mode:?}: status ink {ink:?} on {surface:?} is too faint"
                    );
                }
            }
        }
    }

    /// The kit hover tint and the shared surfaces are central tokens, so
    /// shell/pages/tooltips cannot drift into their own literals again.
    #[test]
    fn shared_color_tokens_are_consistent() {
        assert_eq!(TOOLTIP_BG.a, 1.0);
        assert_eq!(TOOLTIP_FG.a, 1.0);
        const { assert!(MODAL_SCRIM.a > 0.0 && MODAL_SCRIM.a < 1.0) };
        assert_ne!(WHEEL_BASE, WHEEL_HOVER);
        assert_eq!(ON_WHEEL.a, 1.0);
        const { assert!(WHEEL_TITLE_BG.a > 0.0 && WHEEL_TITLE_BG.a < 1.0) };
        for color in [
            PRIMARY_HOVER,
            PRIMARY_ACTIVE,
            DANGER_HOVER,
            DANGER_ACTIVE,
            SUCCESS_HOVER,
            SUCCESS_ACTIVE,
            WARNING_HOVER,
            WARNING_ACTIVE,
        ] {
            assert_eq!(color.a, 1.0, "kit colors must be opaque");
        }
        assert_ne!(KIT_HOVER_DARK, KIT_HOVER_LIGHT);
        assert_eq!(kit_hover(ThemeMode::Dark), KIT_HOVER_DARK);
        assert_eq!(kit_hover(ThemeMode::Light), KIT_HOVER_LIGHT);
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            assert!(
                contrast(ON_WHEEL, WHEEL_BASE) >= 4.5,
                "{mode:?}: wheel ink is unreadable"
            );
            assert!(
                contrast(ON_WHEEL_DIM, WHEEL_HOVER) >= 3.0,
                "{mode:?}: dim wheel ink is unreadable on hover"
            );
        }
    }

    /// Surfaces must stay distinguishable from one another, otherwise cards,
    /// borders and selection read as one flat wash.
    #[test]
    fn surfaces_are_distinct_from_the_page() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let t = tokens(mode);
            assert_ne!(t.bg, t.card, "{mode:?}: card matches the page");
            assert_ne!(t.card, t.sidebar, "{mode:?}: sidebar matches the card");
            assert!(
                contrast(t.bg, t.border) > 1.05,
                "{mode:?}: border invisible on the page"
            );
            assert!(
                contrast(ACCENT, t.card) >= 3.0,
                "{mode:?}: accent unusable on a card"
            );
        }
    }
}
