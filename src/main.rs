#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Composition root for Wheelie.
//!
//! This binary only parses the CLI, initializes logging, owns the runtime and
//! starts the UI. No layout or OS-specific logic belongs here.

mod cli;

use anyhow::Result;
use clap::Parser;
use wheelie::app::{Runtime, logging};
use wheelie::ui;

fn main() -> Result<()> {
    let _cli = cli::Cli::parse();
    let _log_guard = logging::init()?;
    let mut runtime = Runtime::new()?;

    let result = ui::run(runtime.handle());
    runtime.shutdown();
    result
}
