//! Application layer for Wheelie.
//!
//! This crate orchestrates the domain and platform adapters. It must not depend
//! on eframe or any graphical toolkit: views consume [`RuntimeHandle`] through
//! `wheelie-ui`.

pub mod error;
pub mod logging;
pub mod runtime;

pub use error::AppError;
pub use runtime::{AppCommand, AppEvent, Runtime, RuntimeHandle, RuntimeSnapshot, RuntimeState};
