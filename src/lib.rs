//! Wheelie — cross-platform knob utility with a Windows Wheel-inspired radial UI.
//!
//! The crate is organized as a single package with layered modules:
//!
//! ```text
//! core      pure domain: config, keys, actions, macros, gestures, i18n
//! platform  operating-system adapters
//! app       runtime orchestration and services
//! ui        eframe views and widgets
//! ```
//!
//! Dependency direction is strictly `core → platform → app → ui`. The binary in
//! `main.rs` only composes these modules.

pub mod app;
pub mod core;
pub mod platform;
pub mod ui;
