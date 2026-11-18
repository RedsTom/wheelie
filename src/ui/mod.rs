//! UI layer: gpui shell plus the legacy eframe views still being ported.
//!
//! `run` boots the gpui application (`shell`); `app`, `theme`, `widgets` and
//! the eframe-typed pages remain until the F4 cleanup removes eframe.

pub mod app;
pub mod assets;
pub mod controls;
pub mod gestures;
pub mod icons;
pub mod iconset;
pub mod overlay;
pub mod pages;
pub mod services;
pub mod settings;
pub mod shell;
pub mod style;
pub mod theme;
pub mod widgets;
pub mod window;

pub use shell::run;
pub use theme::{ACCENT, ThemeMode, WARNING};
pub use widgets::card;
