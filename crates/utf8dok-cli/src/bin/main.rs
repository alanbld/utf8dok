//! utf8dok CLI binary entry point
//!
//! This is a thin wrapper that calls the library's `run_cli()` function.

use anyhow::Result;
use utf8dok_cli::run_cli;

fn main() -> Result<()> {
    run_cli()
}
