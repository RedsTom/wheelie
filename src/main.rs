#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Composition root for Wheelie.
//!
//! This binary only parses the CLI, initializes logging, owns the runtime and
//! starts the UI. No layout or OS-specific logic belongs here.

mod cli;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let _cli = cli::Cli::parse();
    let _log_guard = wheelie_app::logging::init()?;
    let mut runtime = wheelie_app::Runtime::new()?;

    let result = wheelie_ui::run(runtime.handle());
    runtime.shutdown();
    result
}
