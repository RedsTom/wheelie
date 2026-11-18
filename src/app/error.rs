//! Application-layer errors.

use crate::core::ConfigError;
use crate::platform::PlatformError;
use thiserror::Error;

/// Error surfaced by the orchestration layer.
#[derive(Debug, Error)]
pub enum AppError {
    /// A platform adapter failed.
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),

    /// The configuration document is invalid.
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The runtime channel is closed.
    #[error("the Wheelie runtime is stopped")]
    RuntimeStopped,
}
