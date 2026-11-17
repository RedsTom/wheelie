//! Stable application metadata.

/// User-facing application name.
pub const APP_NAME: &str = "Wheelie";

/// Technical identifier (macOS bundle id, eframe app id, storage keys).
pub const APP_ID: &str = "io.github.redstom.wheelie";

/// Version taken from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Public repository URL.
pub const REPOSITORY_URL: &str = "https://github.com/RedsTom/wheelie";
