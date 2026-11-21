//! Operating-system adapters for Wheelie.
//!
//! This module is the only place that may touch OS-native APIs. It exposes
//! capabilities, paths and the hotkey/input/autostart services used by the
//! application layer.

pub mod autostart;
pub mod brightness;
pub mod error;
pub mod guard;
pub mod hotkeys;
pub mod info;
pub mod input;
pub mod instance;
pub mod key_map;
pub mod media_session;
// The optional feature is declared outside this module; keep plain builds
// quiet when it is not present in the manifest.
#[allow(unexpected_cfgs)]
#[cfg(any(test, feature = "test-doubles"))]
pub mod mock;
pub mod paths;
pub mod tray;
pub mod update;
pub mod volume;
pub mod window_frame;

pub use crate::core::PlatformInfo;
pub use autostart::{AutoStartService, is_enabled as autostart_is_enabled, set as autostart_set};
pub use error::PlatformError;
pub use guard::InjectionGuard;
pub use hotkeys::{GlobalHotkeyService, HotkeySource};
pub use info::info;
pub use input::{EnigoInputSink, InputSink};
pub use instance::InstanceGuard;
pub use key_map::{to_enigo_key, to_enigo_modifiers, to_global_code, to_global_modifiers};
pub use paths::{config_dir, data_dir, ensure_log_dir, log_dir, log_path};
pub use tray::{TrayCommand, TrayService};
pub use update::latest_stable_release;
