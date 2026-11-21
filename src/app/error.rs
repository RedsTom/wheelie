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

    /// The runtime channel is closed or the action worker is unhealthy.
    #[error("the Wheelie runtime is stopped")]
    RuntimeStopped,

    /// Another edit replaced the configuration while this one was being saved.
    #[error("the configuration changed while it was being saved")]
    StaleUpdate,

    /// A requested action set does not exist.
    #[error("unknown action set at index {0}")]
    UnknownActionSet(usize),
}
