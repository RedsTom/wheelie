#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Composition root for Wheelie.
//!
//! This binary only parses the CLI, initializes logging, acquires the single
//! instance, owns the runtime and starts the UI. No layout or OS-specific logic
//! belongs here.

mod cli;

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn};
use wheelie::app::{Runtime, logging};
use wheelie::core::APP_ID;
use wheelie::platform::InstanceGuard;
use wheelie::ui;

fn main() -> Result<()> {
    let _cli = cli::Cli::parse();
    let _log_guard = logging::init()?;

    let instance = InstanceGuard::acquire(APP_ID)?;
    if !instance.is_single() {
        warn!("another Wheelie instance is already running");
        info!("exiting the second instance");
        return Ok(());
    }

    let mut runtime = Runtime::from_executable(std::env::current_exe()?)?;
    let result = ui::run(runtime.handle());
    runtime.shutdown();
    result
}
