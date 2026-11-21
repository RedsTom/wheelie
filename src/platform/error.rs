//! Platform adapter errors.

use thiserror::Error;

/// Source error from a platform backend, boxed so this type stays portable.
pub type BoxedSource = Box<dyn std::error::Error + Send + Sync>;

/// Error returned by a platform adapter.
///
/// New variants may be added without a breaking change, so callers must not
/// match this enum exhaustively.
#[derive(Debug, Error)]
#[non_exhaustive]
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

    /// The global hotkey backend refused an operation.
    #[error("global hotkey error: {0}")]
    Hotkey(#[source] BoxedSource),

    /// COM initialisation or a COM call failed.
    #[error("COM error: {0}")]
    Com(#[source] BoxedSource),

    /// The tray adapter could not build or update the icon.
    #[error("tray error: {0}")]
    Tray(String),

    /// The start-at-login adapter refused an operation.
    #[error("start-at-login error: {0}")]
    Autostart(String),

    /// The release feed could not be read.
    #[error("release feed error: {0}")]
    Feed(String),

    /// An opaque operating-system error.
    #[error("{0}")]
    Other(String),
}
