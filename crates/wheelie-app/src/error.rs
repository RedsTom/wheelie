//! Application-layer errors.

use thiserror::Error;
use wheelie_platform::PlatformError;

/// Error surfaced by the orchestration layer.
#[derive(Debug, Error)]
pub enum AppError {
    /// A platform adapter failed.
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),

    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The runtime channel is closed.
    #[error("the Wheelie runtime is stopped")]
    RuntimeStopped,
}
