//! Operating-system adapters for Wheelie.
//!
//! This module is the only place that may touch OS-native APIs. It exposes
//! capabilities, paths and the hotkey/input/autostart services used by the
//! application layer.

pub mod autostart;
pub mod error;
pub mod guard;
pub mod hotkeys;
pub mod info;
pub mod input;
pub mod instance;
pub mod key_map;
#[cfg(test)]
pub mod mock;
pub mod paths;
pub mod tray;

pub use crate::core::PlatformInfo;
pub use autostart::AutoStartService;
pub use error::PlatformError;
pub use guard::InjectionGuard;
pub use hotkeys::{GlobalHotkeyService, HotkeySource};
pub use info::info;
pub use input::{EnigoInputSink, InputSink};
pub use instance::InstanceGuard;
pub use key_map::{to_enigo_key, to_enigo_modifiers, to_global_code, to_global_modifiers};
pub use paths::{config_dir, data_dir, log_dir};
pub use tray::{TrayCommand, TrayService};
