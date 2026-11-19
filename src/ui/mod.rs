//! UI layer: gpui shell plus the legacy eframe views still being ported.
//!
//! `run` boots the gpui application (`shell`); `app`, `theme`, `widgets` and
//! the eframe-typed pages remain until the F4 cleanup removes eframe. The
//! first-launch introduction lives in the gpui shell and uses the same widgets
//! as the settings pages.

pub mod app;
pub mod assets;
pub mod capture;
pub mod controls;
pub mod gestures;
pub mod icons;
pub mod iconset;
pub mod osd_window;
pub mod overlay;
pub mod overlay_window;
pub mod pages;
pub mod services;
pub mod settings;
pub mod shell;
pub mod style;
pub mod theme;
pub mod update_popup;
pub mod wheel_paint;
pub mod widgets;
pub mod window;

pub use shell::run;
pub use theme::{ACCENT, ThemeMode, WARNING};
pub use widgets::card;
