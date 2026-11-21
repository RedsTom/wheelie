//! Stable application metadata shared by every layer.
//!
//! [`AUTOSTART_FLAG`] is the contract between the launcher used by the
//! autostart entry and the command-line parser in the application layer.

/// Command-line flag used when the operating system starts Wheelie at login.
pub const AUTOSTART_FLAG: &str = "--autostart";

/// User-facing application name.
pub const APP_NAME: &str = "Wheelie";

/// Technical identifier (macOS bundle id, instance lock, storage keys).
pub const APP_ID: &str = "io.github.redstom.wheelie";

/// Version taken from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Public repository URL.
pub const REPOSITORY_URL: &str = "https://github.com/RedsTom/wheelie";
