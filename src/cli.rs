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
//!   - `render`: PDF, text, or HTML written to `--output`
//! - `1` — document parsed as JSON but failed schema validation
//! - `2` — usage error, IO error, malformed JSON, unknown theme,
//!   unknown format, or Typst render error

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::{
    THEMES, ThemeResolveError, ValidationError, compile_html_resolved, compile_text_resolved,
    compile_theme_resolved, resolve_theme, validate_value,
};

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
    /// Render a JSON Resume document to PDF, plain text, or HTML via
    /// the named theme.
    ///
    /// `--theme` is optional for all formats. PDF and text default to
    /// `text-minimal`; HTML defaults to `html-minimal`. `--theme` also
    /// accepts a path to a local `.typ` file — either relative
    /// (`./resume.typ`), absolute (`/abs/path/resume.typ`), or any
    /// string ending in `.typ` or containing a path separator — in
    /// which case the file's bytes are loaded at invocation time and
    /// run under the same Typst sandbox bundled themes do. Single
    /// `.typ` files only for now; directory-based local themes land
    /// in a follow-up on issue #41. To force a bare name with no
    /// path-like signals to resolve as a local file, prefix it with
    /// `./` or give it a `.typ` extension.
    ///
    /// HTML output uses Typst's experimental HTML export; output shape
    /// may shift across ferrocv releases when Typst is bumped. The CLI
    /// surface itself is stable. See `research/44-html-viability.md`.
    ///
    /// Exit codes:
    /// - 0 — rendered successfully; output written to --output
    /// - 1 — JSON parsed but failed schema validation
    /// - 2 — usage error, IO error, parse error, unknown theme, or
    ///   render error
    Render {
        /// Path to a JSON Resume document. Reads stdin if omitted.
        path: Option<PathBuf>,
        /// Theme name or local `.typ` file path. Bundled names (see
        /// `ferrocv themes list`) resolve out of the compile-time
        /// registry; anything ending in `.typ` or containing a path
        /// separator loads from the local filesystem. Optional for
        /// all formats: PDF and text default to `text-minimal`; HTML
        /// defaults to `html-minimal`.
        #[arg(long)]
        theme: Option<String>,
        /// Output format: `pdf`, `text`, or `html`. Defaults to `pdf`.
        /// HTML output is experimental upstream; its shape may shift
        /// when Typst is bumped.
        #[arg(long, default_value = "pdf")]
        format: Format,
        /// Output file path. Parent directories are created as needed.
        /// Defaults to `dist/resume.pdf` for `--format pdf`,
        /// `dist/resume.txt` for `--format text`, and
        /// `dist/resume.html` for `--format html`.
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
    /// List themes bundled with this build.
    ///
    /// `themes list` prints theme names one per line, sorted
    /// lexicographically, to stdout with no decoration — a stable
    /// machine-readable contract.
    ///
    /// The nested-verb form (`themes list` rather than bare `themes`)
    /// leaves room for a sibling `themes install <spec>` subcommand
    /// when issue #41 adds remote-fetchable themes.
    Themes {
        #[command(subcommand)]
        command: ThemesCommands,
    },
}

/// Subcommands of `ferrocv themes`.
///
/// Nested-verb structure reserves space for a future
/// `themes install <spec>` sibling (see issue #41).
#[derive(Debug, Subcommand)]
enum ThemesCommands {
    /// List registered theme names, one per line, sorted.
    List,
}

/// Output formats supported by `ferrocv render`.
///
/// Phase 2 ships PDF, plain text, and HTML. HTML uses Typst's
/// upstream-experimental HTML export; its output shape may shift
/// across `ferrocv` releases when Typst is bumped.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Format {
    Pdf,
    Text,
    Html,
}

/// Resolve which theme name to use given the format and the optional
/// `--theme` argument.
///
/// PDF and text default to the native `text-minimal` theme. HTML
/// defaults to the semantic-HTML native theme `html-minimal`. An
/// explicit `--theme` always wins. See CONSTITUTION §3 for why each
/// format gets its own native default rather than a single shared
/// anchor.
fn resolve_theme_name(format: Format, requested: Option<&str>) -> &str {
    match requested {
        Some(name) => name,
        None => match format {
            Format::Html => "html-minimal",
            Format::Pdf | Format::Text => "text-minimal",
        },
    }
}

/// Default output path for a given format.
///
/// Centralized so the CLI's defaulting logic and any future docs/tests
/// agree on a single source of truth.
fn default_output_path(format: Format) -> PathBuf {
    match format {
        Format::Pdf => PathBuf::from("dist/resume.pdf"),
        Format::Text => PathBuf::from("dist/resume.txt"),
        Format::Html => PathBuf::from("dist/resume.html"),
    }
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
        } => run_render(path.as_deref(), theme.as_deref(), format, output.as_deref()),
        Commands::Themes { command } => match command {
            ThemesCommands::List => run_themes_list(),
        },
    }
}

/// Print the names of every theme registered with this build, one per
/// line, sorted lexicographically ascending, to stdout.
///
/// This is the machine-readable contract: no headers, no decoration,
/// no extra whitespace. Shell pipelines depend on stability here.
///
/// Writes go through a locked `stdout` handle with explicit error
/// handling rather than `println!` — a broken pipe (e.g.
/// `ferrocv themes list | head`) is a normal IO error here, not a
/// panic. Per the module-level exit-code contract, unrecoverable
/// stdout write failures exit with code 2.
fn run_themes_list() -> Result<ExitCode> {
    let mut names: Vec<&'static str> = THEMES.iter().map(|t| t.name).collect();
    // `sort_unstable` is fine — theme names are unique, so stability
    // on equal keys is moot.
    names.sort_unstable();

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    for name in names {
        if let Err(err) = writeln!(stdout, "{name}") {
            eprintln!("error: failed to write theme list to stdout: {err}");
            return Ok(ExitCode::from(2));
        }
    }
    Ok(ExitCode::SUCCESS)
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

    // Step 3: validate. On failure, a summary header plus one indented
    // diagnostic per error to stderr.
    match validate_value(&value) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(errors) => {
            report_validation_errors(&errors, "");
            Ok(ExitCode::from(1))
        }
    }
}

/// Print schema validation errors to stderr with a summary header.
///
/// `suffix` is appended to the header line (after the error count) so
/// `render` can add "; no output written" without `validate` having to
/// lie about emitting an output.
fn report_validation_errors(errors: &[ValidationError], suffix: &str) {
    let n = errors.len();
    let plural = if n == 1 { "" } else { "s" };
    eprintln!("error: schema validation failed ({n} error{plural}){suffix}");
    for err in errors {
        eprintln!("  {err}");
    }
}

fn run_render(
    path: Option<&Path>,
    theme_name: Option<&str>,
    format: Format,
    output: Option<&Path>,
) -> Result<ExitCode> {
    // Step 1: resolve theme name first. Every format now has a default
    // (`text-minimal` for PDF/text, `html-minimal` for HTML), so this
    // is infallible — an explicit `--theme` overrides, otherwise the
    // native default applies.
    let theme_name = resolve_theme_name(format, theme_name);

    // Step 2: read input. IO failures bubble up via anyhow and main
    // maps them to exit code 2 (same as validate).
    let input = read_input(path)?;

    // Step 3: parse JSON.
    let value: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: {err}");
            return Ok(ExitCode::from(2));
        }
    };

    // Step 4: validate. Render is defined to run validate first so
    // users get a clean schema diagnostic before any Typst noise. The
    // header calls out the render-specific consequence (no output
    // written) so a terse validator message doesn't read as a warning.
    if let Err(errors) = validate_value(&value) {
        report_validation_errors(&errors, "; no output written");
        return Ok(ExitCode::from(1));
    }

    // Step 5: resolve theme. Accepts three spec shapes — bundled
    // name, local `.typ` path, or `@preview/...` spec — and returns a
    // ResolvedTheme the compile pipeline can consume without caring
    // which shape the user supplied. Errors carry enough context for
    // a single-line stderr message; we match on variants only to
    // preserve the pre-#41 "available themes: ..." hint on unknown
    // bundled names.
    let theme = match resolve_theme(theme_name) {
        Ok(t) => t,
        Err(err) => {
            match &err {
                ThemeResolveError::NotFound { available, .. } => {
                    eprintln!("error: {err}");
                    let mut names: Vec<&'static str> = available.clone();
                    names.sort_unstable();
                    eprintln!("available themes: {}", names.join(", "));
                }
                _ => {
                    eprintln!("error: {err}");
                }
            }
            return Ok(ExitCode::from(2));
        }
    };

    // Step 6: format dispatch. PDF returns bytes; text and HTML both
    // return a String which we convert to UTF-8 bytes for the shared
    // write path below.
    let bytes: Vec<u8> = match format {
        Format::Pdf => match compile_theme_resolved(&theme, &value) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("{err}");
                return Ok(ExitCode::from(2));
            }
        },
        Format::Text => match compile_text_resolved(&theme, &value) {
            Ok(text) => text.into_bytes(),
            Err(err) => {
                eprintln!("{err}");
                return Ok(ExitCode::from(2));
            }
        },
        Format::Html => match compile_html_resolved(&theme, &value) {
            Ok(html) => html.into_bytes(),
            Err(err) => {
                eprintln!("{err}");
                return Ok(ExitCode::from(2));
            }
        },
    };

    // Step 7: write output. Default path depends on format; parent
    // directories are created as needed. Overwrites without prompting
    // — this is a build tool.
    let out_path: PathBuf = output
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(format));
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
