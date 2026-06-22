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
//!   - `tailor`: derived JSON Resume written to `--output`/stdout
//! - `1` — document parsed as JSON but failed schema validation
//! - `2` — usage error (incl. malformed projection flag value), IO
//!   error, malformed JSON, unknown theme, unknown format, or Typst
//!   render error

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde_json::Value;

use crate::{
    ProjectionError, ProjectionSpec, RedactSet, THEMES, ThemeResolveError, ValidationError,
    compile_html_resolved, compile_text_resolved, compile_theme_resolved, project, resolve_theme,
    validate_value,
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
    /// `--theme` is optional for all formats. PDF defaults to `classic`,
    /// text defaults to `text-minimal`, HTML defaults to `html-minimal`.
    /// `--theme` also accepts a path to a local `.typ` file — either relative
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
        /// all formats: PDF defaults to `classic`, text to `text-minimal`,
        /// HTML to `html-minimal`.
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
        /// Mechanical projection filters. When any are present, the
        /// document is projected (CONSTITUTION §7) before rendering;
        /// with none, render behaves exactly as it would on the raw
        /// input. Equivalent to `ferrocv tailor … | ferrocv render`.
        #[command(flatten)]
        projection: ProjectionArgs,
    },
    /// Project a master JSON Resume into a derived, narrower cut.
    ///
    /// `tailor` runs the projection stage (CONSTITUTION §7) and stops —
    /// it emits a derived document that is itself valid JSON Resume,
    /// which you can inspect, commit, or pipe straight into `render`
    /// (`ferrocv tailor master.json --since 2015 | ferrocv render`).
    /// The master is read unmodified; only the derived output reflects
    /// the filters.
    ///
    /// Reads the master from PATH, or from stdin when PATH is omitted
    /// (matching `render`/`validate`). Writes the derived document to
    /// `--output <file>`, or to stdout when `--output` is omitted, so it
    /// composes in a pipe. All diagnostics go to stderr unconditionally,
    /// so stdout always carries only the JSON document.
    ///
    /// Caution: with no `--output`, the full derived resume — including
    /// any PII not removed by `--redact` — is printed to stdout. Prefer
    /// `--output <file>` for unattended or shared/recorded contexts.
    ///
    /// Supports the curated `--audience` filter and the mechanical
    /// `--since` / `--max-bullets` / `--redact` filters.
    ///
    /// Exit codes:
    /// - 0 — projected; derived document written to --output/stdout
    /// - 1 — master parsed but failed schema validation, or an
    ///   `x-ferrocv.highlights` tag array is misaligned with its
    ///   `highlights`
    /// - 2 — usage error (bad flag value), IO error, or parse error
    Tailor {
        /// Path to the master JSON Resume. Reads stdin if omitted.
        path: Option<PathBuf>,
        #[command(flatten)]
        projection: ProjectionArgs,
        /// Output file path for the derived document. Parent directories
        /// are created as needed. Writes to stdout if omitted.
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
/// `themes install` is the single, enumerated network-permitted entry
/// point per CONSTITUTION.md §6.1 (post-Stage-B amendment); it is
/// gated behind the `install` Cargo feature so the default build
/// contains no network-capable code at all. `themes list` is
/// unconditional.
#[derive(Debug, Subcommand)]
enum ThemesCommands {
    /// List registered theme names, one per line, sorted.
    List,
    /// Download a Typst Universe package into the local cache so
    /// later `render` invocations can resolve `@preview/<name>:<version>`
    /// offline.
    ///
    /// Spec format: `@preview/<name>:<version>` (e.g.
    /// `@preview/basic-resume:0.2.8`). Only the `@preview/` namespace
    /// is accepted in v1; other namespaces are rejected with a clear
    /// error.
    ///
    /// Fetches `https://packages.typst.org/preview/<name>-<version>.tar.gz`
    /// over HTTPS (TLS-only integrity; the registry does not publish
    /// checksums or signatures), extracts into a sibling temp
    /// directory, verifies the tarball's `typst.toml` matches the
    /// spec, and atomically renames the staged directory onto its
    /// final cache path.
    ///
    /// Transitive resolution: any `@preview/<name>:<version>` packages
    /// imported by the requested package's source are fetched and cached
    /// recursively. A summary of the transitive deps installed (or already
    /// cached) is printed to stderr after the primary's status line.
    /// Cycles in declared imports terminate cleanly. A transitive install
    /// failure (404, malformed tarball, etc.) leaves the primary's cache
    /// entry in place so a re-run after fixing the transitive does not
    /// re-fetch the parent.
    ///
    /// Cache location:
    /// - Default: `{dirs::cache_dir()}/ferrocv/packages/preview/<name>/<version>/`
    ///   (Linux: `$XDG_CACHE_HOME or $HOME/.cache/...`,
    ///   macOS: `$HOME/Library/Caches/...`,
    ///   Windows: `%LOCALAPPDATA%\...`).
    /// - Override: set `FERROCV_CACHE_DIR=/some/path` to write to
    ///   `/some/path/packages/preview/<name>/<version>/` instead.
    ///
    /// v1 has no cache eviction: delete the cache directory with
    /// `rm -rf` to reclaim space.
    ///
    /// Exit codes:
    /// - 0: installed successfully (or already cached — idempotent).
    /// - 2: invalid spec, HTTP failure, extraction failure, or
    ///   manifest mismatch.
    #[cfg(feature = "install")]
    Install {
        /// `@preview/<name>:<version>` spec of the package to install.
        spec: String,
    },
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

/// Shared mechanical projection flags (issue #148), attached to both
/// `tailor` and `render` via `#[command(flatten)]`.
///
/// Defining the flags once and feeding them to the single
/// [`crate::project`] transform is what keeps the two surfaces
/// equivalent (ADR 0005): `render --since X` and `tailor --since X |
/// render` run the same code.
#[derive(Debug, Clone, Args)]
struct ProjectionArgs {
    /// Keep only content tagged for this audience under `x-ferrocv`
    /// (curated selection). Untagged content is kept for every audience;
    /// tagged content is kept only for the audiences it lists. Takes
    /// exactly one value.
    #[arg(long, value_name = "NAME")]
    audience: Option<String>,
    /// Drop `work` entries that ended before this ISO 8601 date
    /// (`YYYY`, `YYYY-MM`, or `YYYY-MM-DD`). Entries with no end date
    /// (ongoing roles) are always kept. A malformed value is a usage
    /// error.
    #[arg(long)]
    since: Option<String>,
    /// Cap each entry's `highlights` list at N bullets, keeping the
    /// first N by position. `0` removes all highlights.
    #[arg(long, value_name = "N")]
    max_bullets: Option<usize>,
    /// Redact a named set of PII fields. `pii` removes
    /// `basics.location`, `basics.phone`, and `basics.email`.
    #[arg(long, value_enum)]
    redact: Option<RedactArg>,
}

impl ProjectionArgs {
    /// Translate the parsed CLI flags into a library [`ProjectionSpec`].
    fn to_spec(&self) -> ProjectionSpec {
        ProjectionSpec {
            audience: self.audience.clone(),
            since: self.since.clone(),
            max_bullets: self.max_bullets,
            redact: self.redact.map(|r| match r {
                RedactArg::Pii => RedactSet::Pii,
            }),
        }
    }
}

/// Map a [`ProjectionError`] to its process exit code.
///
/// A malformed flag *value* is a usage error (exit 2), consistent with
/// `--since banana` and an unknown `--redact` value. A misaligned
/// `x-ferrocv.highlights` tag array is a defect in the master *document*,
/// not the invocation, so it shares the schema-failure exit code (1) the
/// way a structurally-invalid master already does.
fn projection_exit_code(err: &ProjectionError) -> u8 {
    match err {
        ProjectionError::InvalidSince(_) => 2,
        ProjectionError::HighlightsTagMismatch { .. } => 1,
    }
}

/// The `--redact` value vocabulary. A fixed enum so clap rejects unknown
/// values as usage errors (exit 2) rather than silently ignoring them.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum RedactArg {
    /// Standard PII contact fields in `basics`.
    Pii,
}

/// Resolve which theme name to use given the format and the optional
/// `--theme` argument.
///
/// PDF defaults to the native PDF-first theme `classic`; text defaults to
/// the extraction-tuned native `text-minimal`; HTML defaults to the
/// semantic-HTML native `html-minimal`. An explicit `--theme` always wins.
/// See CONSTITUTION §3 for why each format gets its own native default
/// rather than a single shared anchor.
fn resolve_theme_name(format: Format, requested: Option<&str>) -> &str {
    match requested {
        Some(name) => name,
        None => match format {
            Format::Pdf => "classic",
            Format::Text => "text-minimal",
            Format::Html => "html-minimal",
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
            projection,
        } => run_render(
            path.as_deref(),
            theme.as_deref(),
            format,
            output.as_deref(),
            &projection.to_spec(),
        ),
        Commands::Tailor {
            path,
            projection,
            output,
        } => run_tailor(path.as_deref(), &projection.to_spec(), output.as_deref()),
        Commands::Themes { command } => match command {
            ThemesCommands::List => run_themes_list(),
            #[cfg(feature = "install")]
            ThemesCommands::Install { spec } => run_themes_install(&spec),
        },
    }
}

/// Run `ferrocv themes install <spec>`.
///
/// Gated behind the `install` Cargo feature — if the binary was built
/// without `--features install`, the `Install` variant does not
/// exist and clap rejects the subcommand with its own "unknown
/// subcommand" error (exit 2).
///
/// Network boundary: this is the ONLY function in the entire CLI that
/// is allowed to make a network call (CONSTITUTION.md §6.1
/// post-Stage-B amendment). It lives in a `#[cfg(feature = "install")]`
/// block so the compiler refuses to build it into the default binary.
#[cfg(feature = "install")]
fn run_themes_install(spec: &str) -> Result<ExitCode> {
    use crate::install::{self, InstallError, pipeline::InstallOutcome};

    let parsed = match install::spec::parse_spec(spec) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: {err}");
            return Ok(ExitCode::from(2));
        }
    };

    match install::install_with_transitive(&parsed) {
        Ok(summary) => {
            let primary_path = summary.primary.path().clone();
            // One-line path on stdout for scripting; human-readable
            // summary on stderr so `$(ferrocv themes install ...)`
            // captures just the primary's path. Transitive dep paths
            // intentionally do NOT appear on stdout — preserves the
            // existing single-path scripting contract. Mirrors the
            // locked-stdout error handling in `run_themes_list` so a
            // broken pipe surfaces as a clean exit-2, not a panic.
            {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                if let Err(err) = writeln!(stdout, "{}", primary_path.display()) {
                    eprintln!("error: failed to write install path to stdout: {err}");
                    return Ok(ExitCode::from(2));
                }
            }
            match &summary.primary {
                InstallOutcome::Installed { .. } => {
                    eprintln!(
                        "installed @preview/{}:{} into {}",
                        parsed.name,
                        parsed.version,
                        primary_path.display(),
                    );
                }
                InstallOutcome::AlreadyCached { .. } => {
                    eprintln!(
                        "@preview/{}:{} already cached at {}",
                        parsed.name,
                        parsed.version,
                        primary_path.display(),
                    );
                }
            }
            // Summary of transitive deps, if any. Suppressed entirely
            // when zero transitives so older scripts that grep stderr
            // for the primary's `installed`/`already cached` line keep
            // working unchanged.
            if !summary.transitive.is_empty() {
                // "resolved" rather than "installed" because the list
                // mixes fresh installs and cache hits — see the per-dep
                // `[installed|cached]` tag for the actual outcome.
                eprintln!(
                    "also resolved {} transitive dep(s):",
                    summary.transitive.len(),
                );
                for (dep_spec, outcome) in &summary.transitive {
                    let tag = match outcome {
                        InstallOutcome::Installed { .. } => "installed",
                        InstallOutcome::AlreadyCached { .. } => "cached",
                    };
                    eprintln!(
                        "  @preview/{}:{} -> {} [{}]",
                        dep_spec.name,
                        dep_spec.version,
                        outcome.path().display(),
                        tag,
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(InstallError::TransitiveDepFailed {
            parent,
            child,
            source,
        }) => {
            eprintln!(
                "error: failed to install transitive dep {child} required by {parent}: {source}",
            );
            // Per the prompt-001 contract the primary's cache entry is
            // left in place on a transitive failure. Tell the user
            // where to find it so they don't think they need to
            // manually clean up before retrying. `package_cache_dir`
            // is a pure path computation; if even that fails (e.g.
            // CacheDirUnresolved) we silently skip the note rather
            // than emit a misleading path.
            if let Ok(p) = install::cache::package_cache_dir(&parsed.name, &parsed.version)
                && p.is_dir()
            {
                eprintln!(
                    "note: primary @preview/{}:{} remains cached at {}; \
                     rerun after fixing the transitive",
                    parsed.name,
                    parsed.version,
                    p.display(),
                );
            }
            Ok(ExitCode::from(2))
        }
        Err(err) => {
            eprintln!("error: {err}");
            // Give the user a hint about inspectable state. For
            // filesystem-touching errors, point at the cache root so
            // they can investigate. For `CacheDirUnresolved` (the one
            // case the cache root *isn't* discoverable), surface the
            // env-var override instead.
            match &err {
                InstallError::Extract { .. } | InstallError::Io { .. } => {
                    if let Ok(root) = install::cache::preview_cache_root() {
                        eprintln!("cache root: {}", root.display());
                    }
                }
                InstallError::CacheDirUnresolved => {
                    eprintln!("hint: set FERROCV_CACHE_DIR to override the cache location");
                }
                _ => {}
            }
            Ok(ExitCode::from(2))
        }
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
    projection: &ProjectionSpec,
) -> Result<ExitCode> {
    // Step 0: validate projection flags before touching the document, so
    // a malformed flag value (e.g. `--since banana`) is a usage error
    // (exit 2) and wins deterministically over an unrelated schema
    // failure in the input (exit 1).
    if let Err(err) = projection.validate() {
        eprintln!("error: {err}; no output written");
        return Ok(ExitCode::from(2));
    }

    // Step 1: resolve theme name first. Every format now has a default
    // (`classic` for PDF, `text-minimal` for text, `html-minimal` for
    // HTML), so this is infallible — an explicit `--theme` overrides,
    // otherwise the native default applies.
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

    // Step 4b: project. When any projection flag is set, run the same
    // transform `tailor` runs (ADR 0005) and render the derived
    // document; with no flags the input flows through untouched so
    // flagless `render` is unchanged. The master is validated above; the
    // derived document is valid JSON Resume by construction.
    let value = if projection.is_noop() {
        value
    } else {
        match project(&value, projection) {
            Ok(derived) => derived,
            Err(err) => {
                eprintln!("error: {err}; no output written");
                return Ok(ExitCode::from(projection_exit_code(&err)));
            }
        }
    };

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

/// Run `ferrocv tailor`.
///
/// Projects the master with `spec` and writes the derived JSON Resume to
/// `output` (or stdout when `None`). Diagnostics always go to stderr so
/// stdout carries only the document (ADR 0005). Exit-code contract
/// matches the module header: 0 ok, 1 schema-invalid master, 2
/// usage/IO/parse error.
fn run_tailor(
    path: Option<&Path>,
    spec: &ProjectionSpec,
    output: Option<&Path>,
) -> Result<ExitCode> {
    // Step 0: validate projection flags before touching the document, so
    // a malformed flag value (e.g. `--since banana`) is a usage error
    // (exit 2) rather than being masked by a schema failure (exit 1) in
    // the master.
    if let Err(err) = spec.validate() {
        eprintln!("error: {err}; no output written");
        return Ok(ExitCode::from(2));
    }

    // Step 1: read input (IO errors → exit 2 via main's anyhow mapping).
    let input = read_input(path)?;

    // Step 2: parse JSON.
    let value: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: {err}");
            return Ok(ExitCode::from(2));
        }
    };

    // Step 3: validate the master before projecting, so a bad master is
    // a clean schema diagnostic rather than a confusing projected dump.
    if let Err(errors) = validate_value(&value) {
        report_validation_errors(&errors, "; no output written");
        return Ok(ExitCode::from(1));
    }

    // Step 4: project. A malformed flag value (e.g. a non-date `--since`)
    // is a usage error (exit 2); a misaligned audience tag array is a
    // defect in the master document, treated like a schema failure (exit
    // 1). See `projection_exit_code`.
    let derived = match project(&value, spec) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("error: {err}; no output written");
            return Ok(ExitCode::from(projection_exit_code(&err)));
        }
    };

    // Step 5: serialize. Pretty-printed for human inspection and clean
    // diffs; the derived document is valid JSON Resume.
    let mut json = match serde_json::to_string_pretty(&derived) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: failed to serialize derived document: {err}");
            return Ok(ExitCode::from(2));
        }
    };
    json.push('\n');

    // Step 6: write to file or stdout.
    match output {
        Some(out_path) => {
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
            if let Err(err) = std::fs::write(out_path, json.as_bytes()) {
                eprintln!(
                    "error: failed to write output file {}: {err}",
                    out_path.display()
                );
                return Ok(ExitCode::from(2));
            }
        }
        None => {
            // Locked stdout with explicit error handling so a broken
            // pipe (`ferrocv tailor … | head`) is a clean exit-2, not a
            // panic — mirrors `run_themes_list`.
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            if let Err(err) = stdout.write_all(json.as_bytes()) {
                eprintln!("error: failed to write derived document to stdout: {err}");
                return Ok(ExitCode::from(2));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The per-format native defaults (issue #188): PDF renders via the
    /// PDF-first `classic` theme, while text and HTML keep their
    /// extraction- and semantic-HTML-tuned defaults. CONSTITUTION §3:
    /// each format gets its own native default.
    #[test]
    fn resolve_theme_name_uses_per_format_native_defaults() {
        assert_eq!(resolve_theme_name(Format::Pdf, None), "classic");
        assert_eq!(resolve_theme_name(Format::Text, None), "text-minimal");
        assert_eq!(resolve_theme_name(Format::Html, None), "html-minimal");
    }

    /// An explicit `--theme` always wins over the per-format default,
    /// for every format.
    #[test]
    fn resolve_theme_name_explicit_request_wins() {
        assert_eq!(
            resolve_theme_name(Format::Pdf, Some("text-minimal")),
            "text-minimal"
        );
        assert_eq!(resolve_theme_name(Format::Text, Some("classic")), "classic");
        assert_eq!(
            resolve_theme_name(Format::Html, Some("typst-jsonresume-cv")),
            "typst-jsonresume-cv"
        );
    }
}
