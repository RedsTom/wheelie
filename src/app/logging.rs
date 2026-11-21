//! `tracing` initialization to stderr and a daily log file.

use super::error::AppError;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_FILTER: &str = "info,wheelie=debug,wheelie_core=debug";

/// Initializes logging. Keep the returned guard alive until shutdown.
///
/// Initialization is idempotent: a second call (for example from a test) only
/// warns instead of panicking. An invalid `RUST_LOG` is reported before the
/// subscriber exists and falls back to the default filter.
pub fn init() -> Result<WorkerGuard, AppError> {
    let log_dir = crate::platform::log_dir()?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "wheelie.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(error) => {
            eprintln!("invalid RUST_LOG directive, using the default filter: {error}");
            EnvFilter::new(DEFAULT_FILTER)
        }
    };

    if tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false),
        )
        .try_init()
        .is_err()
    {
        eprintln!("logging was already initialized");
    }

    tracing::info!(
        version = crate::core::VERSION,
        os = crate::platform::info().os,
        "Wheelie started"
    );

    Ok(guard)
}
