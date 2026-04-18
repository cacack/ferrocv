//! Command-line interface for `ferrocv`.
//!
//! This module owns argument parsing (via `clap`), input acquisition
//! (file or stdin), and exit-code handling. The library in
//! [`crate::validate`] stays CLI-free so it can be reused by tests and
//! future embedders.
//!
//! Exit codes (contractual):
//! - `0` — document validates against JSON Resume v1.0.0
//! - `1` — document parsed as JSON but failed schema validation
//! - `2` — usage error, IO error, or malformed JSON

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::validate_value;

/// Render JSON Resume documents via embedded Typst.
#[derive(Debug, Parser)]
#[command(name = "ferrocv", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Validate a JSON Resume document against the bundled schema.
    ///
    /// Reads from PATH if given, otherwise from stdin. Exits 0 on
    /// valid input, 1 on schema violations (diagnostics on stderr),
    /// and 2 on IO or parse errors.
    Validate {
        /// Path to a JSON Resume document. Reads stdin if omitted.
        path: Option<PathBuf>,
    },
}

/// Entry point invoked from `main`.
///
/// Returns an `ExitCode` rather than calling `std::process::exit` so
/// destructors run normally.
pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate { path } => run_validate(path.as_deref()),
    }
}

fn run_validate(path: Option<&std::path::Path>) -> Result<ExitCode> {
    // Step 1: read input. IO failures are exit code 2.
    let input =
        match path {
            Some(p) => std::fs::read_to_string(p)
                .with_context(|| format!("failed to read {}", p.display()))?,
            None => std::io::read_to_string(std::io::stdin())
                .context("failed to read JSON from stdin")?,
        };

    // Step 2: parse JSON. Parse failures are exit code 2 and print a
    // single `error: ...` line to stderr rather than a validation list.
    let value: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: {err}");
            return Ok(ExitCode::from(2));
        }
    };

    // Step 3: validate. On failure, one diagnostic per error to stderr.
    match validate_value(&value) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(errors) => {
            for err in errors {
                eprintln!("{err}");
            }
            Ok(ExitCode::from(1))
        }
    }
}
