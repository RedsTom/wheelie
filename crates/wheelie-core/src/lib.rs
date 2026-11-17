//! Pure domain model for Wheelie.
//!
//! This crate must stay independent from eframe, operating-system APIs and
//! file-system side effects. It owns the configuration model, key chords,
//! actions, macros, gesture detection and translations.

pub mod app_info;
pub mod i18n;
pub mod platform;

pub use app_info::{APP_ID, APP_NAME, REPOSITORY_URL, VERSION};
pub use i18n::{Language, TextKey, tr};
pub use platform::PlatformInfo;
