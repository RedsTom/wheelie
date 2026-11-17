//! Platform adapter errors.

use thiserror::Error;

/// Error returned by a platform adapter.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// The requested capability is not available on this platform.
    #[error("feature not available on this platform: {0}")]
    Unsupported(&'static str),

    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Input simulation failed.
    #[error("input simulation failed: {0}")]
    Input(#[from] enigo::InputError),

    /// An opaque operating-system error.
    #[error("{0}")]
    Other(String),
}
