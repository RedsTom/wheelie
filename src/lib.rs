//! Wheelie — cross-platform knob utility with a Windows Wheel-inspired radial UI.
//!
//! The crate is organized as a single package with layered modules:
//!
//! ```text
//! core      pure domain: config, keys, actions, macros, gestures, i18n
//! platform  operating-system adapters
//! app       runtime orchestration and services
//! ui        gpui shell, settings pages and the radial wheel
//! ```
//!
//! Dependencies point inward: ui → app → platform → core; core depends on no
//! other layer. The binary in `main.rs` only composes these modules.

pub mod app;
pub mod assets;
pub mod core;
pub mod platform;
pub mod ui;
