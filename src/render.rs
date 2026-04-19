//! In-process Typst compilation to PDF.
//!
//! Wraps the embedded `typst` crate behind a small library surface.
//! Two public entry points:
//!
//! - [`compile_pdf`] — compile a single Typst source string against a
//!   JSON Resume value. Intended for smoke tests, one-off rendering,
//!   and callers who supply Typst source directly.
//! - [`compile_theme`] — compile a [`crate::theme::Theme`] (a bundle
//!   of Typst files shipped with `ferrocv`) against a JSON Resume
//!   value. This is the path the CLI `render` subcommand uses.
//!
//! Both run the real Typst compiler in-process (no subprocess, no
//! shell-out) and return the produced PDF byte vector.
//!
//! # Constitutional commitments
//!
//! - **§2 — Embed Typst, never subprocess it.** The whole compilation
//!   path is `typst::compile(&world)` followed by `typst_pdf::pdf(...)`,
//!   both linked statically. No `std::process::Command`, no shelling
//!   out to the `typst` CLI, ever.
//! - **§6.1 — No network calls at render time.** The [`FerrocvWorld`]
//!   does not implement a package resolver. Any `FileId` carrying a
//!   `PackageSpec` (i.e. `@preview/...` imports) returns
//!   [`FileError::Package(PackageError::NotFound(_))`]. There is no
//!   code path by which Typst can reach the network from inside
//!   `compile_pdf` or `compile_theme`. A test in `tests/render.rs`
//!   enforces this.
//! - **§6.4 — Themes run under Typst's native sandbox, nothing more.**
//!   We do not add filesystem-wide access, shell-escape, or any
//!   custom capabilities. The World exposes exactly the virtual
//!   files the caller registers plus the always-present
//!   `/resume.json` slot wired to the caller-supplied JSON Resume
//!   bytes.
//!
//! # Fonts
//!
//! Fonts are bundled at build time via the `typst-assets` crate's
//! `fonts` feature (DejaVu Sans Mono, Libertinus Serif, New Computer
//! Modern, etc.). We do **not** scan the system font directory.
//! Bundling trades binary size (~20 MB) for cross-host reproducibility
//! — the same source compiles to the same PDF on Ubuntu, macOS, and
//! Windows CI runners.
//!
//! # Warnings
//!
//! Typst's `compile` returns a `Warned<Result<...>>`. For Phase 1 we
//! discard the warnings; only fatal diagnostics surface as
//! [`RenderError`]. Surfacing warnings is intentionally deferred until
//! a caller (a theme adapter or the `render` CLI subcommand) has a
//! concrete need for them.
//!
//! # Library only
//!
//! This module knows nothing about clap, files, stdin, or exit codes.
//! Those concerns live in `crate::cli`.
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use serde_json::Value;
use typst::diag::{FileError, FileResult, PackageError, Warned};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_pdf::PdfOptions;

use crate::theme::Theme;

/// Virtual path of the single-file source that [`compile_pdf`] serves.
const MAIN_PATH: &str = "/main.typ";
/// Virtual path the JSON Resume bytes are served under. Typst sources
/// reach them via `json("/resume.json")`.
const RESUME_JSON_PATH: &str = "/resume.json";

/// One Typst diagnostic, flattened into a renderer-agnostic shape.
///
/// Only `message` is currently exposed because the upstream `Span`
/// type carries Typst-internal interner state that we do not want to
/// leak through `ferrocv`'s public API. Span info can be added later
/// as a stringified location if a caller needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDiagnostic {
    /// Human-readable error message from the Typst compiler.
    pub message: String,
}

impl fmt::Display for RenderDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)
    }
}

/// A failure to compile a Typst document to PDF.
///
/// Carries one or more [`RenderDiagnostic`]s flattened from Typst's
/// internal `SourceDiagnostic` vector. The `Display` impl prints one
/// `error: <message>` line per diagnostic, mirroring the spirit of
/// [`crate::ValidationError`]'s `Display` impl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    diagnostics: Vec<RenderDiagnostic>,
}

impl RenderError {
    /// Borrow the underlying diagnostics for callers that want
    /// structured access (e.g. to render their own error UI rather
    /// than the default `Display` line-per-diagnostic format).
    pub fn diagnostics(&self) -> &[RenderDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Defensive: surface a synthetic message if we somehow ended
        // up with an empty diagnostic list. Better than printing
        // nothing.
        if self.diagnostics.is_empty() {
            return write!(f, "error: render failed without a diagnostic");
        }
        for (i, diag) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{diag}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RenderError {}

/// Compile a single Typst source string to PDF bytes, in-process.
///
/// `source` is a complete Typst document. `data` is the JSON Resume
/// document; the source can read it via `json("/resume.json")` (the
/// leading slash is required — Typst's path resolution is absolute
/// against the World root).
///
/// On success, the returned vector starts with the PDF magic bytes
/// `%PDF-`. On failure, returns [`RenderError`] with one or more
/// diagnostics describing what the compiler rejected.
///
/// Convenience wrapper around [`compile_theme`]: builds a World whose
/// only theme file is `source` registered at `/main.typ`.
///
/// # Warnings
///
/// Non-fatal Typst warnings are **discarded**; only errors surface.
/// See the module-level documentation for the rationale.
pub fn compile_pdf(source: &str, data: &Value) -> Result<Vec<u8>, RenderError> {
    let world = FerrocvWorld::from_single_source(source, data);
    render_world(&world)
}

/// Compile a [`Theme`] bundle to PDF bytes against a JSON Resume
/// value.
///
/// Every `(virtual_path, bytes)` pair in `theme.files` is registered
/// in the World. Typst starts compilation from `theme.entrypoint`;
/// relative imports inside theme sources resolve against the
/// entrypoint's virtual directory, which is why it's the caller's
/// responsibility to register all referenced files under a shared
/// prefix (see `crate::theme` for the convention).
///
/// On success, the returned vector starts with the PDF magic bytes
/// `%PDF-`. On failure, returns [`RenderError`] with one or more
/// diagnostics describing what the compiler rejected — e.g. a
/// reference to a field the supplied `data` does not carry, or a
/// `@preview/...` import that the World refuses to resolve
/// (CONSTITUTION §6.1).
pub fn compile_theme(theme: &Theme, data: &Value) -> Result<Vec<u8>, RenderError> {
    let world = FerrocvWorld::from_theme(theme, data);
    render_world(&world)
}

/// Shared compile + PDF-serialize path used by both entry points.
fn render_world(world: &FerrocvWorld) -> Result<Vec<u8>, RenderError> {
    // `typst::compile` returns Warned { output, warnings }. Drop
    // warnings — see module doc for rationale.
    let Warned {
        output,
        warnings: _,
    } = typst::compile::<PagedDocument>(world);
    let document = output.map_err(diagnostics_to_error)?;

    // Serialize to PDF. `typst_pdf::pdf` can also emit diagnostics
    // (e.g. for unsupported font features) — surface them via the
    // same RenderError path.
    typst_pdf::pdf(&document, &PdfOptions::default()).map_err(diagnostics_to_error)
}

/// Map a Typst `EcoVec<SourceDiagnostic>` into our public error type.
fn diagnostics_to_error<I>(diags: I) -> RenderError
where
    I: IntoIterator,
    I::Item: AsSourceDiagnostic,
{
    let diagnostics: Vec<RenderDiagnostic> = diags
        .into_iter()
        .map(|d| RenderDiagnostic {
            message: d.message_string(),
        })
        .collect();
    RenderError { diagnostics }
}

/// Helper trait so we can accept either an owned or borrowed
/// `SourceDiagnostic` iterator without depending on its concrete type
/// at the call site (`typst::compile` and `typst_pdf::pdf` both
/// produce `EcoVec<SourceDiagnostic>` here, so this is mild
/// future-proofing rather than current necessity).
trait AsSourceDiagnostic {
    fn message_string(&self) -> String;
}

impl AsSourceDiagnostic for typst::diag::SourceDiagnostic {
    fn message_string(&self) -> String {
        self.message.to_string()
    }
}

/// The [`World`] implementation that backs [`compile_pdf`] and
/// [`compile_theme`].
///
/// One concrete struct, no trait objects, no builder. Holds:
/// - `entrypoint` — interned [`FileId`] for the entrypoint source.
/// - `entrypoint_source` — the parsed [`Source`] for the entrypoint.
/// - `resume_id` — interned [`FileId`] for `/resume.json`.
/// - `resume_bytes` — the JSON Resume bytes Typst will read via
///   `json("/resume.json")`.
/// - `theme_files` — a map from every other theme file's [`FileId`]
///   to its raw bytes. Typst hits this map for every non-entrypoint
///   theme import.
///
/// Fonts and the standard library are stored in process-wide
/// [`OnceLock`]s because they are immutable and expensive to build.
struct FerrocvWorld {
    entrypoint: FileId,
    entrypoint_source: Source,
    resume_id: FileId,
    resume_bytes: Bytes,
    theme_files: HashMap<FileId, Bytes>,
}

impl FerrocvWorld {
    /// Build a World whose only theme file is a single inline source
    /// registered at [`MAIN_PATH`].
    fn from_single_source(source_text: &str, data: &Value) -> Self {
        let entrypoint = FileId::new(None, VirtualPath::new(MAIN_PATH));
        let entrypoint_source = Source::new(entrypoint, source_text.to_owned());
        Self::assemble(entrypoint, entrypoint_source, HashMap::new(), data)
    }

    /// Build a World from a [`Theme`] bundle.
    ///
    /// Every file in `theme.files` is interned as a [`FileId`] and
    /// stored in the byte map, except the entrypoint itself which is
    /// additionally parsed into a [`Source`] (Typst needs a parsed
    /// `Source` for the main file — see [`World::main`]).
    fn from_theme(theme: &Theme, data: &Value) -> Self {
        let entrypoint = FileId::new(None, VirtualPath::new(theme.entrypoint));
        let mut theme_files: HashMap<FileId, Bytes> = HashMap::with_capacity(theme.files.len());
        let mut entrypoint_text: Option<String> = None;

        for (path, bytes) in theme.files {
            let id = FileId::new(None, VirtualPath::new(path));
            // The entrypoint gets parsed into a Source below. We also
            // keep its bytes in the map so a `file()` lookup works
            // (Typst occasionally reads the main file as raw bytes
            // for, e.g., span reporting; costs nothing to populate).
            if id == entrypoint {
                // UTF-8 is a hard requirement for any `.typ` file
                // Typst can parse. An invalid-UTF-8 theme source is
                // a packaging bug; panic is the right response.
                let text = std::str::from_utf8(bytes)
                    .expect("theme entrypoint must be valid UTF-8 Typst source")
                    .to_owned();
                entrypoint_text = Some(text);
            }
            theme_files.insert(id, Bytes::new(bytes.to_vec()));
        }

        let text = entrypoint_text.expect("Theme.entrypoint must appear as a key in Theme.files");
        let entrypoint_source = Source::new(entrypoint, text);

        Self::assemble(entrypoint, entrypoint_source, theme_files, data)
    }

    /// Common tail of both constructors — wires the `/resume.json`
    /// slot from `data` and returns the assembled World.
    fn assemble(
        entrypoint: FileId,
        entrypoint_source: Source,
        theme_files: HashMap<FileId, Bytes>,
        data: &Value,
    ) -> Self {
        let resume_id = FileId::new(None, VirtualPath::new(RESUME_JSON_PATH));
        // `serde_json::to_vec` is infallible for `Value` — the Value
        // tree by construction never fails to serialize. Unwrap is
        // an invariant assertion, not a possible Typst behavior.
        let bytes =
            serde_json::to_vec(data).expect("serde_json::Value must always serialize to bytes");
        let resume_bytes = Bytes::new(bytes);
        Self {
            entrypoint,
            entrypoint_source,
            resume_id,
            resume_bytes,
            theme_files,
        }
    }
}

/// Cached library; built once per process.
fn shared_library() -> &'static LazyHash<Library> {
    static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();
    LIBRARY.get_or_init(|| LazyHash::new(Library::default()))
}

/// Cached `(FontBook, fonts)` pair; built once per process.
///
/// Fonts come exclusively from `typst-assets` — no system font
/// scanning. See module doc for the reproducibility rationale.
fn shared_fonts() -> &'static (LazyHash<FontBook>, Vec<Font>) {
    static FONTS: OnceLock<(LazyHash<FontBook>, Vec<Font>)> = OnceLock::new();
    FONTS.get_or_init(|| {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let book = FontBook::from_fonts(&fonts);
        (LazyHash::new(book), fonts)
    })
}

impl World for FerrocvWorld {
    fn library(&self) -> &LazyHash<Library> {
        shared_library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &shared_fonts().0
    }

    fn main(&self) -> FileId {
        self.entrypoint
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.entrypoint {
            return Ok(self.entrypoint_source.clone());
        }
        // §6.1: package-rooted imports are rejected at the World
        // layer. No package resolver, no network call.
        if let Some(spec) = id.package() {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        }
        // Theme files: parse on demand. Caching is deferred (§5) —
        // a fresh `Source` per lookup is cheap enough for Phase 1.
        if let Some(bytes) = self.theme_files.get(&id) {
            let text = std::str::from_utf8(bytes.as_slice())
                .map_err(|_| FileError::NotFound(id.vpath().as_rootless_path().into()))?;
            return Ok(Source::new(id, text.to_owned()));
        }
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.resume_id {
            return Ok(self.resume_bytes.clone());
        }
        // §6.1: same package-rejection rule as `source`. Anything
        // claiming to be inside a `@preview/...` package is a network
        // request we refuse to make.
        if let Some(spec) = id.package() {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        }
        if let Some(bytes) = self.theme_files.get(&id) {
            return Ok(bytes.clone());
        }
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        shared_fonts().1.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        // We return None deliberately. Reasoning:
        //
        // - Reproducibility matters for golden-file tests landing
        //   with the first theme adapter (#12). Returning the wall
        //   clock would make the same source compile to different
        //   PDF bytes on different days.
        // - Returning a fixed datetime now would bake an arbitrary
        //   choice into the public surface before any theme has
        //   asked for one. Per CONSTITUTION §5 ("simple now,
        //   iterate later"), defer the decision until a real caller
        //   needs `datetime.today()`.
        // - With None, Typst's `datetime.today()` returns an error
        //   if a theme calls it — a clear, fail-loud signal we can
        //   then address by passing in an explicit reference date.
        None
    }
}
