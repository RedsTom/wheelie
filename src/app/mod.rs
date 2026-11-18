//! Application layer for Wheelie.
//!
//! This module orchestrates the domain and platform adapters. It must not depend
//! on eframe or any graphical toolkit: views consume [`RuntimeHandle`] through
//! the `ui` module.

pub mod action_worker;
pub mod config_store;
pub mod error;
pub mod logging;
pub mod runtime;

pub use action_worker::execute_action;
pub use config_store::{CONFIG_FILE_NAME, ConfigStore, LoadedConfig};
pub use error::AppError;
pub use runtime::{
    AppCommand, AppEvent, Indicator, Runtime, RuntimeHandle, RuntimeSnapshot, RuntimeState,
};
