//! Command-line interface.

use clap::Parser;

/// Command-line options for Wheelie.
#[derive(Debug, Parser)]
#[command(
    name = "wheelie",
    version,
    about = "Configurable radial wheel for keyboard knobs",
    long_about = None
)]
pub struct Cli {}
