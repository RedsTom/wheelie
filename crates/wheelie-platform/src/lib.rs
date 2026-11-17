//! Operating-system adapters for Wheelie.
//!
//! This crate is the only place that may touch OS-native APIs. It exposes
//! capabilities, paths and (from M2) hotkey/input/autostart services.

pub mod error;
pub mod info;
pub mod paths;

pub use error::PlatformError;
pub use info::info;
pub use paths::{config_dir, data_dir, log_dir};
pub use wheelie_core::PlatformInfo;
