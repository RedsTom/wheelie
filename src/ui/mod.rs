//! UI layer: the gpui application shell, its pages and the radial wheel.
//!
//! Everything here renders through gpui: `run` boots the shell, `pages` holds
//! the settings pages, `controls` the shared widgets, `style` the color tokens
//! and `wheel_paint` the radial wheel. The first-launch introduction lives in
//! the shell and uses the same widgets as the settings pages.
//!
//! This layer owns no business rules and makes no OS calls: it reads and writes
//! configuration through [`crate::app::RuntimeHandle`] and consumes
//! [`crate::core`] types.

pub mod assets;
pub mod capture;
pub mod controls;
pub mod gestures;
pub mod iconset;
pub mod nav;
pub mod osd_window;
pub mod overlay_window;
pub mod pages;
pub mod services;
pub mod shell;
pub mod style;
pub mod update_popup;
pub mod wheel;
pub mod wheel_paint;
pub mod window;

pub use shell::run;
