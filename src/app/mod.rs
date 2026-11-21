//! Application layer for Wheelie.
//!
//! This module orchestrates the domain and platform adapters. It must not depend
//! on gpui or any graphical toolkit: views consume [`RuntimeHandle`] through
//! the `ui` module.

pub mod action_worker;
pub mod config_store;
pub mod error;
pub mod logging;
pub mod runtime;

pub use action_worker::{
    ActionError, InjectionTracker, discard_injected, execute_action, filter_injected,
};
pub use config_store::{CONFIG_FILE_NAME, ConfigStore, LoadedConfig};
pub use error::AppError;
pub use runtime::{
    AppCommand, AppEvent, CoverImageFormat, CoverView, Indicator, MEDIA_DISPLAY_DURATION,
    PlayingView, Runtime, RuntimeHandle, RuntimeSnapshot, RuntimeState, VolumeView,
};

/// Re-exported so the UI can consume tray commands without depending on the
/// platform module directly.
pub use crate::platform::TrayCommand;
