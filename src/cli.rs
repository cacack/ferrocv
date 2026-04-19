//! Command-line interface for `ferrocv`.
//!
//! This module owns argument parsing (via `clap`), input acquisition
//! (file or stdin), and exit-code handling. The library in
//! [`crate::validate`] and [`crate::render`] stays CLI-free so it can
//! be reused by tests and future embedders.
//!
//! Exit codes (contractual, shared across subcommands):
//! - `0` — operation succeeded
//!   - `validate`: document is valid
//!   - `render`: PDF written to `--output`
//! - `1` — document parsed as JSON but failed schema validation
//! - `2` — usage error, IO error, malformed JSON, unknown theme,
//!   unknown format, or Typst render error

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::{THEMES, compile_theme, find_theme, validate_value};

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
    /// Render a JSON Resume document to PDF via the named theme.
    ///
    /// Exit codes:
    /// - 0 — rendered successfully; PDF written to --output
    /// - 1 — JSON parsed but failed schema validation
    /// - 2 — IO error, parse error, unknown theme, or render error
    Render {
        /// Path to a JSON Resume document. Reads stdin if omitted.
        path: Option<PathBuf>,
        /// Theme name. See the registered themes in `ferrocv::THEMES`.
        #[arg(long)]
        theme: String,
        /// Output format. Only `pdf` is supported in Phase 1.
        #[arg(long, default_value = "pdf")]
        format: Format,
        /// Output file path. Parent directories are created as needed.
        /// Defaults to `dist/resume.pdf`.
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
}

/// Output formats supported by `ferrocv render`.
///
/// Phase 1 ships PDF only. HTML and plain text land in Phase 2 (#14)
/// — adding them is a matter of new variants plus a matching arm in
/// [`run_render`]'s format dispatch, not a refactor.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Format {
    Pdf,
    // TODO Phase 2 (#14): Html, Text
}

/// Entry point invoked from `main`.
///
/// Returns an `ExitCode` rather than calling `std::process::exit` so
/// destructors run normally.
pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate { path } => run_validate(path.as_deref()),
        Commands::Render {
            path,
            theme,
            format,
            output,
        } => run_render(path.as_deref(), &theme, format, output.as_deref()),
    }
}

fn run_validate(path: Option<&Path>) -> Result<ExitCode> {
    // Step 1: read input. IO failures are exit code 2 (via main's
    // anyhow→2 mapping).
    let input = read_input(path)?;

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

fn run_render(
    path: Option<&Path>,
    theme_name: &str,
    format: Format,
    output: Option<&Path>,
) -> Result<ExitCode> {
    // Step 1: read input. IO failures bubble up via anyhow and main
    // maps them to exit code 2 (same as validate).
    let input = read_input(path)?;

    // Step 2: parse JSON.
    let value: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: {err}");
            return Ok(ExitCode::from(2));
        }
    };

    // Step 3: validate. Render is defined to run validate first so
    // users get a clean schema diagnostic before any Typst noise.
    if let Err(errors) = validate_value(&value) {
        for err in errors {
            eprintln!("{err}");
        }
        return Ok(ExitCode::from(1));
    }

    // Step 4: resolve theme. Unknown names list the alternatives so
    // users know what they could have typed.
    let theme = match find_theme(theme_name) {
        Some(t) => t,
        None => {
            eprintln!("error: unknown theme `{theme_name}`");
            let names: Vec<&'static str> = THEMES.iter().map(|t| t.name).collect();
            eprintln!("available themes: {}", names.join(", "));
            return Ok(ExitCode::from(2));
        }
    };

    // Step 5: format dispatch. Phase 1 ships PDF only.
    let bytes = match format {
        Format::Pdf => match compile_theme(theme, &value) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("{err}");
                return Ok(ExitCode::from(2));
            }
        },
        // TODO Phase 2 (#14): Html, Text
    };

    // Step 6: write output. Default path is `dist/resume.pdf`; parent
    // directories are created as needed. Overwrites without prompting
    // — this is a build tool.
    let out_path: PathBuf = output
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dist/resume.pdf"));
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "error: failed to create output directory {}: {err}",
            parent.display()
        );
        return Ok(ExitCode::from(2));
    }
    if let Err(err) = std::fs::write(&out_path, &bytes) {
        eprintln!(
            "error: failed to write output file {}: {err}",
            out_path.display()
        );
        return Ok(ExitCode::from(2));
    }

    Ok(ExitCode::SUCCESS)
}

/// Read JSON input from a file path or stdin.
///
/// Shared by both subcommands; IO failures are surfaced via anyhow so
/// the caller can map them to exit code 2.
fn read_input(path: Option<&Path>) -> Result<String> {
    match path {
        Some(p) => {
            std::fs::read_to_string(p).with_context(|| format!("failed to read {}", p.display()))
        }
        None => std::io::read_to_string(std::io::stdin()).context("failed to read JSON from stdin"),
    }
}
