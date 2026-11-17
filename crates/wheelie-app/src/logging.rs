//! `tracing` initialization to stderr and a daily log file.

use crate::error::AppError;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes logging. Keep the returned guard alive until shutdown.
pub fn init() -> Result<WorkerGuard, AppError> {
    let log_dir = wheelie_platform::log_dir()?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "wheelie.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wheelie=debug,wheelie_core=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false),
        )
        .init();

    tracing::info!(
        version = wheelie_core::VERSION,
        os = wheelie_platform::info().os,
        "Wheelie started"
    );

    Ok(guard)
}
