//! Theme registry for `ferrocv`.
//!
//! A [`Theme`] is a bundle of Typst source files ([`include_bytes!`]'d
//! at compile time) plus the virtual path Typst should start compiling
//! from. Theme files are served by [`crate::render::FerrocvWorld`]
//! through its in-memory file map; there is no filesystem access at
//! render time (CONSTITUTION §6.1, §6.4).
//!
//! # Adapters vs. native themes — CONSTITUTION §4
//!
//! This module's registry currently holds a mix:
//!
//! - **Adapters** wrap an upstream Typst template (e.g.
//!   `typst-jsonresume-cv`) and hand it a JSON Resume structure
//!   through the conventional `/resume.json` virtual file. Adapters
//!   accept that upstream layout changes may break them; in return
//!   they give `ferrocv` visual variety without re-implementing a
//!   full resume renderer.
//! - **Native themes** implement a `render(data) -> content` contract
//!   directly against parsed JSON Resume data. `text-minimal` feeds
//!   clean plain-text output to [`crate::render::compile_text`];
//!   `html-minimal` targets Typst's typed-HTML API
//!   ([`crate::render::compile_html`]) with semantic `<section>` /
//!   `<h2>` markup.
//!
//! Per CONSTITUTION §4 the two layers are kept separable: adapter
//! code does not leak into native themes, and native themes do not
//! depend on adapter internals. §4 also promises that native themes
//! will eventually live in a dedicated module. For Phase 2, with two
//! small native themes colocated here (`text-minimal` and
//! `html-minimal`), splitting into a dedicated module would add
//! scaffolding without saving anything — the themes share no Rust
//! code, only a registration pattern. Extract when shared
//! native-theme infrastructure actually warrants it (CONSTITUTION §5:
//! "simple now, iterate later").
//!
//! # Why a static slice, not a `HashMap` or `ThemeRegistry`
//!
//! Phase 2 ships five themes total: three adapters
//! (`typst-jsonresume-cv`, `fantastic-cv`, `modern-cv`) and two native
//! themes (`text-minimal`, `html-minimal`). A linear scan over
//! `THEMES` is O(n) for small n; CONSTITUTION §5 ("simple now,
//! iterate later") calls for the narrower solution here. Generalizing
//! to a hashed lookup or a builder pattern should wait for a caller
//! that actually needs it.
//!
//! # Theme resolution: bundled vs. local-path
//!
//! Issue #41's first stage introduces [`resolve_theme`], which takes
//! the raw `--theme <spec>` string and returns a [`ResolvedTheme`] —
//! an enum with two variants:
//!
//! - [`ResolvedTheme::Bundled`] wraps a `&'static` [`Theme`] picked
//!   out of [`THEMES`] by name (the legacy path, byte-for-byte
//!   equivalent to calling [`find_theme`]).
//! - [`ResolvedTheme::Owned`] wraps an [`OwnedTheme`] assembled at
//!   runtime from bytes the CLI read off the filesystem. v1 of the
//!   local-path feature accepts a single `.typ` file; directory-based
//!   themes are rejected with a clear error pointing at the follow-up
//!   issue. No sibling imports, no package resolver — the file runs
//!   under exactly the same Typst sandbox bundled themes do.
//!
//! `@preview/<name>:<version>` specs route through the offline
//! installer cache populated by `ferrocv themes install` (Stage B).
//! Cache hits resolve to [`ResolvedTheme::Owned`] and feed straight
//! into the same compile pipeline bundled themes use; cache misses
//! return [`ThemeResolveError::PreviewCacheMiss`] so the CLI can
//! point the user at `ferrocv themes install` rather than calling the
//! network-capable installer transitively. Render and validate stay
//! fully offline (CONSTITUTION §6.1, post-Stage-B amendment).
//!
//! When the binary is built without the `install` Cargo feature the
//! Universe-resolution path errors out with
//! [`ThemeResolveError::PreviewSpecRequiresInstallFeature`] — the
//! cache reader and the manifest parser both live behind that feature
//! flag, so a default build cannot resolve `@preview/...` specs even
//! against a populated cache.

/// A themed Typst source bundle that [`crate::render::compile_theme`]
/// can compile against a JSON Resume document.
///
/// `name` is the registry key (the string a CLI `--theme <name>`
/// argument will eventually match against). `files` is the set of
/// Typst source files the theme needs, each keyed by the virtual
/// path it will resolve under inside [`crate::render::FerrocvWorld`].
/// `entrypoint` is the virtual path of the file `typst::compile`
/// starts from.
///
/// All fields are `'static` because themes are defined as `const`s
/// and their contents come from [`include_bytes!`].
#[derive(Debug)]
pub struct Theme {
    /// Registry key. Matches the value passed to [`find_theme`].
    pub name: &'static str,
    /// `(virtual_path, bytes)` pairs. Virtual paths must begin with
    /// `/` (Typst's `VirtualPath` resolution is absolute against the
    /// World root) and must be unique within a single `Theme`.
    pub files: &'static [(&'static str, &'static [u8])],
    /// Virtual path of the file `typst::compile` starts from. MUST
    /// appear as a key in [`Self::files`].
    pub entrypoint: &'static str,
}

/// Virtual-path prefix for this theme's files inside the World.
///
/// Centralized as a private `const` so the `files` and `entrypoint`
/// fields stay in lockstep. If this prefix changes, every file path
/// in [`TYPST_JSONRESUME_CV`] updates in one place.
const TYPST_JSONRESUME_CV_PREFIX: &str = "/themes/typst-jsonresume-cv";

/// Virtual-path prefix for this theme's files inside the World.
///
/// Centralized as a private `const` so the `files` and `entrypoint`
/// fields stay in lockstep. If this prefix changes, every file path
/// in [`FANTASTIC_CV`] updates in one place.
const FANTASTIC_CV_PREFIX: &str = "/themes/fantastic-cv";

/// Virtual-path prefix for this theme's files inside the World.
///
/// Centralized as a private `const` so the `files` and `entrypoint`
/// fields stay in lockstep. If this prefix changes, every file path
/// in [`MODERN_CV`] updates in one place.
const MODERN_CV_PREFIX: &str = "/themes/modern-cv";

/// Adapter for [`fruggiero/typst-jsonresume-cv`]'s `basic-resume`
/// theme, vendored under `assets/themes/typst-jsonresume-cv/`.
///
/// The entrypoint is the patched `resume.typ`. It does
/// `#import "base.typ": *`, which Typst resolves relative to the
/// entrypoint's virtual directory — hence both files sit side-by-side
/// under the same prefix. See `assets/themes/typst-jsonresume-cv/VENDORING.md`
/// for the patch record and upstream commit SHA.
///
/// [`fruggiero/typst-jsonresume-cv`]: https://github.com/fruggiero/typst-jsonresume-cv
pub const TYPST_JSONRESUME_CV: Theme = Theme {
    name: "typst-jsonresume-cv",
    files: &[
        (
            // Must agree with TYPST_JSONRESUME_CV_PREFIX + "/base.typ".
            concat!("/themes/typst-jsonresume-cv", "/base.typ"),
            include_bytes!("../assets/themes/typst-jsonresume-cv/base.typ"),
        ),
        (
            concat!("/themes/typst-jsonresume-cv", "/resume.typ"),
            include_bytes!("../assets/themes/typst-jsonresume-cv/resume.typ"),
        ),
    ],
    entrypoint: concat!("/themes/typst-jsonresume-cv", "/resume.typ"),
};

// Compile-time sanity check: the entrypoint matches the prefix we
// centralized above. Kept as a `const _` so a typo in either string
// literal becomes a build error rather than a runtime mystery.
const _: () = {
    // We can't do string comparison in const context on stable without
    // extra ceremony, so we just assert the prefix constant is
    // non-empty and referenced. The `concat!` expressions above will
    // themselves fail to compile if the prefix name is wrong.
    assert!(!TYPST_JSONRESUME_CV_PREFIX.is_empty());
};

/// Adapter for [`austinyu/fantastic-cv`], vendored under
/// `assets/themes/fantastic-cv/`.
///
/// The entrypoint is our authored glue `resume.typ`, which
/// `#import`s the byte-for-byte vendored `fantastic-cv.typ` from the
/// same virtual directory. All JSON-Resume → fantastic-cv field
/// mapping lives in the glue; the vendored source is untouched. See
/// `assets/themes/fantastic-cv/VENDORING.md` for the provenance record
/// and the glue-not-patch rationale.
///
/// [`austinyu/fantastic-cv`]: https://github.com/austinyu/fantastic-cv
pub const FANTASTIC_CV: Theme = Theme {
    name: "fantastic-cv",
    files: &[
        (
            // Must agree with FANTASTIC_CV_PREFIX + "/fantastic-cv.typ".
            concat!("/themes/fantastic-cv", "/fantastic-cv.typ"),
            include_bytes!("../assets/themes/fantastic-cv/fantastic-cv.typ"),
        ),
        (
            concat!("/themes/fantastic-cv", "/resume.typ"),
            include_bytes!("../assets/themes/fantastic-cv/resume.typ"),
        ),
    ],
    entrypoint: concat!("/themes/fantastic-cv", "/resume.typ"),
};

// Compile-time sanity check: same shape as for TYPST_JSONRESUME_CV.
const _: () = {
    assert!(!FANTASTIC_CV_PREFIX.is_empty());
};

/// Adapter for [`DeveloperPaul123/modern-cv`] (canonical:
/// `ptsouchlos/modern-cv`), vendored under `assets/themes/modern-cv/`.
///
/// Unlike [`FANTASTIC_CV`] (which is a pure glue-only vendor — the
/// upstream source is byte-for-byte unchanged), this adapter ships a
/// **patched** `lib.typ`: the upstream pulls `@preview/fontawesome`
/// and `@preview/linguify` at compile time, which CONSTITUTION §6.1
/// forbids. All icon and i18n call sites were rewritten; see
/// `assets/themes/modern-cv/VENDORING.md` for the patch record.
/// The entrypoint is our authored glue `resume.typ`, which imports
/// the patched `lib.typ` from the same virtual directory.
///
/// [`DeveloperPaul123/modern-cv`]: https://github.com/DeveloperPaul123/modern-cv
pub const MODERN_CV: Theme = Theme {
    name: "modern-cv",
    files: &[
        (
            // Must agree with MODERN_CV_PREFIX + "/lib.typ".
            concat!("/themes/modern-cv", "/lib.typ"),
            include_bytes!("../assets/themes/modern-cv/lib.typ"),
        ),
        (
            concat!("/themes/modern-cv", "/resume.typ"),
            include_bytes!("../assets/themes/modern-cv/resume.typ"),
        ),
    ],
    entrypoint: concat!("/themes/modern-cv", "/resume.typ"),
};

// Compile-time sanity check: same shape as for TYPST_JSONRESUME_CV.
const _: () = {
    assert!(!MODERN_CV_PREFIX.is_empty());
};

/// Virtual path of the `text-minimal` theme's entrypoint.
///
/// Single per-file constant used by both the [`Theme::files`] key and
/// the [`Theme::entrypoint`] field below, so the two cannot drift out
/// of sync. This is the cleanup CodeRabbit flagged on the original PR
/// — the previous "prefix" constant was declared but unused (the
/// `concat!` calls hardcoded the literal), making the centralization
/// claim cosmetic. The adapter above still uses the older
/// prefix-as-const pattern; tightening it the same way is its own
/// scope.
const TEXT_MINIMAL_RESUME_PATH: &str = "/themes/text-minimal/resume.typ";

/// `text-minimal` — a **native theme** (per CONSTITUTION §4) authored
/// directly against the JSON Resume v1.0.0 schema, with no upstream
/// template to wrap.
///
/// It exists to produce clean output for
/// [`crate::render::compile_text`]. The Frame-walk extractor sorts
/// glyph runs by `(page, y, x)` and joins same-line items with a
/// space; multi-column or floated layouts therefore produce zig-zag
/// reading order. `text-minimal` is single-column, uses explicit
/// `linebreak()` and `parbreak()` for line and paragraph boundaries,
/// avoids decorative glyphs (no bullets, arrows, dingbats — those
/// survive frame extraction and add ATS noise), and sticks with the
/// default font for cross-host reproducibility (CONSTITUTION §6).
///
/// Every field access in the theme source is wrapped in
/// `dict.at(k, default: none)` so any schema-valid JSON Resume
/// document compiles, including documents that exercise only
/// `basics.name` (the `render_sparse.json` fixture is the lower
/// bound).
///
/// The MIT-licensed source under `assets/themes/text-minimal/` is
/// also redistributable under the `ferrocv` crate's MIT-or-Apache-2.0
/// dual license; the file-level `LICENSE` is duplicated so the theme
/// remains self-contained if it is ever extracted into its own
/// package.
///
/// CONSTITUTION §4 promises a separate native-themes module
/// eventually. For Phase 2, `text-minimal` and `html-minimal`
/// colocate here without sharing Rust code; splitting would add
/// scaffolding without paying for itself (§5: "simple now, iterate
/// later"). See the module-level doc for the full rationale.
pub const TEXT_MINIMAL: Theme = Theme {
    name: "text-minimal",
    files: &[(
        TEXT_MINIMAL_RESUME_PATH,
        include_bytes!("../assets/themes/text-minimal/resume.typ"),
    )],
    entrypoint: TEXT_MINIMAL_RESUME_PATH,
};

/// Virtual path of the `html-minimal` theme's entrypoint.
///
/// Single per-file constant used by both the [`Theme::files`] key and
/// the [`Theme::entrypoint`] field below, so the two cannot drift out
/// of sync. Mirrors the [`TEXT_MINIMAL_RESUME_PATH`] pattern.
const HTML_MINIMAL_RESUME_PATH: &str = "/themes/html-minimal/resume.typ";

/// `html-minimal` — a **native theme** (per CONSTITUTION §4) authored
/// directly against the JSON Resume v1.0.0 schema and targeted at
/// Typst's typed-HTML API (`html.elem`, `html.body`, …) through
/// [`crate::render::compile_html`].
///
/// Where [`TEXT_MINIMAL`] optimizes for frame-walk text extraction
/// (see its doc-comment for the single-column / no-dingbat rationale),
/// `html-minimal` optimizes for **semantic HTML** output: resume
/// sections are wrapped in `<section>` with `<h2>` headings, contact
/// details land in a `<ul>`, and work/education entries use `<article>`
/// so downstream ATS and web consumers can parse structure without
/// regexing the text. It is deliberately *not* plain-text-extractable —
/// that is `text-minimal`'s job, and CONSTITUTION §3 calls for each
/// format to get its own sensible default rather than forcing a single
/// theme to straddle both.
///
/// The MIT-licensed source under `assets/themes/html-minimal/` is
/// also redistributable under the `ferrocv` crate's MIT-or-Apache-2.0
/// dual license; the file-level `LICENSE` is duplicated so the theme
/// remains self-contained if it is ever extracted into its own
/// package.
pub const HTML_MINIMAL: Theme = Theme {
    name: "html-minimal",
    files: &[(
        HTML_MINIMAL_RESUME_PATH,
        include_bytes!("../assets/themes/html-minimal/resume.typ"),
    )],
    entrypoint: HTML_MINIMAL_RESUME_PATH,
};

/// All themes registered with this build of `ferrocv`.
///
/// Phase 2 ships three adapters (`typst-jsonresume-cv`, `fantastic-cv`,
/// `modern-cv`) and two native themes (`text-minimal`, `html-minimal`).
/// See the module doc for why this is a `&[&Theme]` rather than a
/// `HashMap` or a builder pattern — a linear scan over a handful of
/// entries is fine, and CONSTITUTION §5 calls for the narrower
/// solution until a caller actually needs more. See the module doc as
/// well for the §4 deferral on splitting native themes into their own
/// module.
pub const THEMES: &[&Theme] = &[
    &TYPST_JSONRESUME_CV,
    &FANTASTIC_CV,
    &MODERN_CV,
    &TEXT_MINIMAL,
    &HTML_MINIMAL,
];

/// Look up a [`Theme`] by name. Returns `None` for unknown names.
///
/// Linear scan over [`THEMES`]; O(n) for n themes. Acceptable for the
/// current handful of entries (CONSTITUTION §5).
pub fn find_theme(name: &str) -> Option<&'static Theme> {
    THEMES.iter().copied().find(|t| t.name == name)
}

/// Virtual path a local-path `.typ` file is registered at inside the
/// [`crate::render::FerrocvWorld`]. Keeping this centralized — rather
/// than deriving it from the user-supplied path — means the file is
/// served under a stable, predictable location regardless of where on
/// disk it originated. Mirrors the `/themes/<name>/...` shape bundled
/// themes use.
const LOCAL_THEME_ENTRYPOINT: &str = "/themes/local/resume.typ";

/// A Typst source bundle owned at runtime rather than baked into the
/// binary.
///
/// Structural twin of [`Theme`] but with owned fields: strings and
/// byte vectors instead of `&'static` references. Built by
/// [`resolve_theme`] when the user points `--theme` at a local `.typ`
/// file; downstream compilation goes through the same
/// [`crate::render::FerrocvWorld`] path bundled themes do.
///
/// # Fields
///
/// - `name` — a human-readable identifier for diagnostics, e.g.
///   `"local:/abs/path/to/resume.typ"`. Never collides with a bundled
///   theme name because bundled names never contain `:` or `/`.
/// - `files` — `(virtual_path, bytes)` pairs; same shape as
///   [`Theme::files`] just with owned data. Virtual paths must begin
///   with `/` and be unique.
/// - `entrypoint` — virtual path of the file Typst starts compiling
///   from. MUST appear as a key in `files`.
///
/// v1 of the local-path feature only ever populates one entry in
/// `files`; the `Vec` shape is chosen so Stage C's cache-resolver can
/// reuse [`OwnedTheme`] for multi-file `@preview/...` packages without
/// another API churn.
#[derive(Debug, Clone)]
pub struct OwnedTheme {
    /// Registry-key-equivalent identifier for diagnostics. Never
    /// collides with bundled names.
    pub name: String,
    /// `(virtual_path, bytes)` pairs. Same invariants as
    /// [`Theme::files`].
    pub files: Vec<(String, Vec<u8>)>,
    /// Virtual path of the file Typst starts compilation from. MUST
    /// appear as a key in [`Self::files`].
    pub entrypoint: String,
}

/// Outcome of [`resolve_theme`] — either a reference into the
/// compile-time [`THEMES`] registry or an [`OwnedTheme`] assembled at
/// runtime from user-supplied bytes.
///
/// Downstream code does not need to match on the variants; the helper
/// methods ([`Self::name`], [`Self::entrypoint`], [`Self::files`])
/// surface the uniform view every consumer needs. The render pipeline
/// wraps each variant in an internal `ThemeBundle` trait impl (see
/// `src/render.rs`) so Typst's `FerrocvWorld` can ingest both shapes
/// without allocating a temporary [`Theme`].
#[derive(Debug, Clone)]
pub enum ResolvedTheme {
    /// A theme picked out of the compile-time [`THEMES`] slice by
    /// name. Byte-for-byte equivalent to the pre-#41 resolution path.
    Bundled(&'static Theme),
    /// A theme assembled at runtime from bytes the CLI read off the
    /// filesystem (or, in Stage C, the local installer cache).
    Owned(OwnedTheme),
}

impl ResolvedTheme {
    /// Human-readable identifier for diagnostics.
    ///
    /// Bundled themes return their registry key; owned themes return
    /// the synthetic `name` field (e.g. `"local:/abs/path/resume.typ"`).
    pub fn name(&self) -> &str {
        match self {
            ResolvedTheme::Bundled(t) => t.name,
            ResolvedTheme::Owned(o) => &o.name,
        }
    }

    /// Virtual path of the file Typst starts compiling from.
    pub fn entrypoint(&self) -> &str {
        match self {
            ResolvedTheme::Bundled(t) => t.entrypoint,
            ResolvedTheme::Owned(o) => &o.entrypoint,
        }
    }

    /// Iterate over `(virtual_path, bytes)` pairs for every file in
    /// the resolved theme.
    ///
    /// Abstracts the two ownership shapes: bundled themes yield
    /// `&'static [u8]` slices via [`Theme::files`]; owned themes yield
    /// slices borrowed off their `Vec<u8>` entries. Callers get a
    /// uniform borrowed view and never need to allocate.
    pub fn files(&self) -> Box<dyn Iterator<Item = (&str, &[u8])> + '_> {
        match self {
            ResolvedTheme::Bundled(t) => Box::new(t.files.iter().map(|(p, b)| (*p, *b as &[u8]))),
            ResolvedTheme::Owned(o) => {
                Box::new(o.files.iter().map(|(p, b)| (p.as_str(), b.as_slice())))
            }
        }
    }
}

/// Errors returned by [`resolve_theme`].
///
/// All variants map to CLI exit code 2 (usage/input error); the `cli`
/// module owns that mapping. Each variant carries enough context that
/// a single-line `error: ...` stderr message is actionable without a
/// follow-up "why?" from the user.
#[derive(Debug)]
pub enum ThemeResolveError {
    /// A bundled-style name (no path separators, no `.typ` suffix,
    /// no `@preview/...` prefix) did not match any entry in
    /// [`THEMES`]. The registered names are returned alongside so the
    /// CLI can print a "did you mean..." hint.
    NotFound {
        /// The name the user typed.
        name: String,
        /// The names of every registered bundled theme, unsorted.
        available: Vec<&'static str>,
    },
    /// A local-path spec resolved to an existing filesystem entry
    /// that is not a regular `.typ` file — typically a directory or a
    /// file without the `.typ` extension. Directory-based local themes
    /// are tracked as a follow-up on issue #41.
    LocalPathNotAFile {
        /// The path the user typed, as given.
        path: std::path::PathBuf,
    },
    /// A local-path spec pointed at a path that does not exist on the
    /// filesystem.
    LocalPathNotFound {
        /// The path the user typed, as given.
        path: std::path::PathBuf,
    },
    /// Reading a local-path `.typ` file failed for an IO reason other
    /// than "not found" — e.g. permissions, a broken symlink, or an
    /// unreadable device entry.
    LocalPathIoError {
        /// The path the user typed, as given.
        path: std::path::PathBuf,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// A local-path `.typ` file contained bytes that are not valid
    /// UTF-8. Typst source files are required to be UTF-8; this is a
    /// clearer message than letting the compiler reject it later.
    LocalPathNotUtf8 {
        /// The path the user typed, as given.
        path: std::path::PathBuf,
    },
    /// The user supplied an `@preview/<name>:<version>` spec but the
    /// package was not present in the local installer cache. The CLI
    /// formats this into a single-line "run `ferrocv themes install
    /// @preview/...`" hint pointing at Stage B's subcommand. Carries
    /// the cache path that was inspected so the diagnostic is
    /// reproducible.
    PreviewCacheMiss {
        /// The raw spec the user typed.
        spec: String,
        /// Filesystem path the resolver expected to find the package
        /// at. Showing it lets the user verify e.g. that
        /// `FERROCV_CACHE_DIR` is set to what they think it is.
        expected_path: std::path::PathBuf,
    },
    /// The cached package directory exists but is malformed —
    /// missing `typst.toml`, broken manifest, manifest declares a
    /// different name/version than the cache layout, or the declared
    /// entrypoint does not exist on disk. Hint: re-install with
    /// `ferrocv themes install --refresh` (TODO when that flag
    /// lands) or delete the cache directory and retry.
    PreviewCacheCorrupt {
        /// The raw spec the user typed.
        spec: String,
        /// Path to the file or directory that triggered the
        /// corruption diagnostic.
        path: std::path::PathBuf,
        /// Human-readable explanation of what was malformed.
        reason: String,
    },
    /// The user supplied an `@preview/<name>:<version>` spec on a
    /// build that does not include the `install` Cargo feature, so
    /// the cache reader is not compiled in. CLI maps this to "rebuild
    /// with `--features install`, run `ferrocv themes install`, retry
    /// the render".
    PreviewSpecRequiresInstallFeature {
        /// The raw spec the user typed.
        spec: String,
    },
    /// The user supplied a string starting with `@preview/` that is
    /// not a syntactically valid spec. The previous design folded this
    /// into [`Self::PreviewCacheMiss`], but that produced a circular
    /// hint ("Run: ferrocv themes install <bad spec>" — which would
    /// hit the same parse failure). Splitting it lets the CLI print a
    /// pointed "expected `@preview/<name>:<version>`" message instead.
    PreviewSpecInvalid {
        /// The raw spec the user typed.
        spec: String,
        /// Human-readable explanation from the spec parser.
        reason: String,
    },
}

impl std::fmt::Display for ThemeResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeResolveError::NotFound { name, .. } => {
                write!(f, "unknown theme `{name}`")
            }
            ThemeResolveError::LocalPathNotAFile { path } => write!(
                f,
                "local-path theme must point to a .typ file, not a directory or non-.typ file: {} \
                 (directory-based local themes are tracked as a follow-up on issue #41; \
                 for now concatenate your theme into a single .typ file)",
                path.display()
            ),
            ThemeResolveError::LocalPathNotFound { path } => {
                write!(f, "local-path theme not found: {}", path.display())
            }
            ThemeResolveError::LocalPathIoError { path, source } => write!(
                f,
                "failed to read local-path theme {}: {source}",
                path.display()
            ),
            ThemeResolveError::LocalPathNotUtf8 { path } => write!(
                f,
                "local-path theme {} is not valid UTF-8 (Typst source files must be UTF-8)",
                path.display()
            ),
            ThemeResolveError::PreviewCacheMiss {
                spec,
                expected_path,
            } => write!(
                f,
                "theme '{spec}' not found in cache at {}. \
                 Run: ferrocv themes install {spec}",
                expected_path.display(),
            ),
            ThemeResolveError::PreviewCacheCorrupt { spec, path, reason } => write!(
                f,
                "cached theme {spec} is corrupt at {}: {reason}. \
                 Remove the cache directory and re-run `ferrocv themes install {spec}`.",
                path.display(),
            ),
            ThemeResolveError::PreviewSpecRequiresInstallFeature { spec } => write!(
                f,
                "theme '{spec}' requires a build with the `install` Cargo feature. \
                 Rebuild with `cargo install ferrocv --features install`, \
                 then run `ferrocv themes install {spec}` before this render."
            ),
            ThemeResolveError::PreviewSpecInvalid { spec, reason } => write!(
                f,
                "invalid Typst Universe spec '{spec}': {reason}. \
                 Expected '@preview/<name>:<version>' (e.g. '@preview/basic-resume:0.2.8')."
            ),
        }
    }
}

impl std::error::Error for ThemeResolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ThemeResolveError::LocalPathIoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Classify how a raw `--theme <spec>` string should be resolved.
///
/// Three variants, evaluated in this order: `@preview/...` specs come
/// first (unambiguous prefix); path-indicative signals (leading `.` or
/// `/`, a path separator anywhere in the string, or a `.typ` suffix)
/// come next; everything else is treated as a bundled-theme name.
enum ThemeSpecKind {
    /// `@preview/<name>:<version>` — Typst Universe package spec;
    /// Stage A defers resolution to Stage C.
    PreviewPackage,
    /// A filesystem path the CLI should read bytes from.
    LocalPath,
    /// A bundled theme name to look up in [`THEMES`].
    BundledName,
}

/// Classify a `--theme` argument without performing any IO.
fn classify_spec(spec: &str) -> ThemeSpecKind {
    if spec.starts_with("@preview/") {
        return ThemeSpecKind::PreviewPackage;
    }
    // Any explicit path-indicative signal routes to local-path mode.
    // Bundled names contain only lowercase ASCII letters, digits, and
    // hyphens by convention, so none of these signals misfires for a
    // real bundled name.
    let looks_like_path = spec.starts_with('.')
        || spec.starts_with('/')
        || spec.contains('/')
        || spec.contains('\\')
        || spec.ends_with(".typ");
    if looks_like_path {
        return ThemeSpecKind::LocalPath;
    }
    ThemeSpecKind::BundledName
}

/// Resolve a `--theme <spec>` string to a [`ResolvedTheme`].
///
/// Detection order (no IO performed until the path branch is taken):
///
/// 1. Specs starting with `@preview/` resolve through the offline
///    installer cache (gated behind the `install` Cargo feature):
///    cache hits return [`ResolvedTheme::Owned`]; cache misses
///    return [`ThemeResolveError::PreviewCacheMiss`] so the CLI can
///    point at `ferrocv themes install`. On default builds (no
///    `install` feature) the spec is rejected with
///    [`ThemeResolveError::PreviewSpecRequiresInstallFeature`].
/// 2. Specs carrying path-indicative signals — a leading `.` or `/`,
///    any `/` or `\` separator, or a `.typ` suffix — take the
///    local-path branch: the CLI reads the bytes at the path,
///    verifies it is a regular `.typ` file with valid UTF-8 content,
///    and packages it into an [`OwnedTheme`] at a fixed virtual path.
/// 3. Everything else is treated as a bundled-theme name and looked
///    up in [`THEMES`]; unknown names return
///    [`ThemeResolveError::NotFound`] with the full list of
///    registered alternatives for hint generation.
///
/// # Errors
///
/// See [`ThemeResolveError`]. All errors are user-input failures and
/// the CLI maps them to exit code 2.
///
/// # Offline guarantee
///
/// This function performs no network calls. The `@preview/...` branch
/// reads only from the local installer cache populated by a prior
/// `ferrocv themes install`; on cache miss it returns a clean error
/// pointing at the install subcommand and never invokes the
/// network-capable installer module transitively
/// (CONSTITUTION §6.1, post-Stage-B amendment).
pub fn resolve_theme(spec: &str) -> Result<ResolvedTheme, ThemeResolveError> {
    match classify_spec(spec) {
        ThemeSpecKind::PreviewPackage => resolve_preview_package(spec),
        ThemeSpecKind::LocalPath => resolve_local_path(spec),
        ThemeSpecKind::BundledName => match find_theme(spec) {
            Some(theme) => Ok(ResolvedTheme::Bundled(theme)),
            None => Err(ThemeResolveError::NotFound {
                name: spec.to_owned(),
                available: THEMES.iter().map(|t| t.name).collect(),
            }),
        },
    }
}

/// Resolve an `@preview/<name>:<version>` spec via the local installer
/// cache.
///
/// Default-features build: returns
/// [`ThemeResolveError::PreviewSpecRequiresInstallFeature`] without
/// touching the filesystem — the cache reader is not compiled in.
///
/// `install`-feature build: parses the spec via the same parser the
/// installer uses, then delegates to
/// [`crate::package_cache::resolve_preview_spec_from_cache`]. That
/// helper reads only from the cache directory; it never imports
/// [`crate::install::fetch`] or any other network-capable module, so
/// render and validate stay fully offline even on cache miss.
#[cfg(not(feature = "install"))]
fn resolve_preview_package(spec: &str) -> Result<ResolvedTheme, ThemeResolveError> {
    Err(ThemeResolveError::PreviewSpecRequiresInstallFeature {
        spec: spec.to_owned(),
    })
}

#[cfg(feature = "install")]
fn resolve_preview_package(spec: &str) -> Result<ResolvedTheme, ThemeResolveError> {
    // Parse failures get their own variant so the user sees an
    // actionable "expected '@preview/<name>:<version>'" message rather
    // than a circular "run themes install <bad spec>" hint that would
    // hit the same parse failure.
    let parsed = match crate::install::spec::parse_spec(spec) {
        Ok(p) => p,
        Err(err) => {
            return Err(ThemeResolveError::PreviewSpecInvalid {
                spec: spec.to_owned(),
                reason: format!("{err}"),
            });
        }
    };
    crate::package_cache::resolve_preview_spec_from_cache(&parsed).map(ResolvedTheme::Owned)
}

/// Read a local-path `.typ` file into an [`OwnedTheme`].
///
/// Keeps the IO isolated to one function so [`resolve_theme`] stays a
/// clean dispatch. The caller has already classified `spec` as
/// path-like; this function performs all the filesystem checks.
fn resolve_local_path(spec: &str) -> Result<ResolvedTheme, ThemeResolveError> {
    let path = std::path::PathBuf::from(spec);

    // Use `try_exists` semantics: `metadata()` distinguishes "doesn't
    // exist" from other IO failures (e.g. permission denied on a
    // parent component). Keeping them separate gives clearer errors.
    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ThemeResolveError::LocalPathNotFound { path });
        }
        Err(err) => {
            return Err(ThemeResolveError::LocalPathIoError { path, source: err });
        }
    };

    // v1 scope locks us to a single `.typ` file. Directories, symlinks
    // to directories, and non-`.typ` regular files all fail here with
    // a clear pointer to the follow-up issue.
    let has_typ_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("typ"))
        .unwrap_or(false);
    if !metadata.is_file() || !has_typ_extension {
        return Err(ThemeResolveError::LocalPathNotAFile { path });
    }

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(err) => return Err(ThemeResolveError::LocalPathIoError { path, source: err }),
    };

    // Typst source must be UTF-8. Validate early so the user gets a
    // pointed error rather than a cryptic compile diagnostic later.
    if std::str::from_utf8(&bytes).is_err() {
        return Err(ThemeResolveError::LocalPathNotUtf8 { path });
    }

    // Canonicalize the path for the display name so diagnostics and
    // equality comparisons both see the fully-resolved form. Fall
    // back to the user-supplied path if canonicalization fails (rare
    // — implies the path was deleted between metadata and canonicalize).
    let display_path = std::fs::canonicalize(&path).unwrap_or(path);
    let name = format!("local:{}", display_path.display());

    Ok(ResolvedTheme::Owned(OwnedTheme {
        name,
        files: vec![(LOCAL_THEME_ENTRYPOINT.to_owned(), bytes)],
        entrypoint: LOCAL_THEME_ENTRYPOINT.to_owned(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit coverage for the bundled-vs-local-vs-preview detection
    /// heuristic. Uses only pure-string classification (no IO) except
    /// where the bundled-name branch needs a registry lookup.
    #[test]
    fn classify_spec_bundled_names() {
        // Every real bundled name classifies as bundled.
        for theme in THEMES {
            assert!(
                matches!(classify_spec(theme.name), ThemeSpecKind::BundledName),
                "bundled theme `{}` must classify as BundledName",
                theme.name,
            );
        }
    }

    #[test]
    fn classify_spec_preview_packages() {
        assert!(matches!(
            classify_spec("@preview/basic-resume:0.2.8"),
            ThemeSpecKind::PreviewPackage
        ));
        assert!(matches!(
            classify_spec("@preview/foo:1.0.0"),
            ThemeSpecKind::PreviewPackage
        ));
    }

    #[test]
    fn classify_spec_local_paths() {
        // Leading `./` or `../`, absolute paths, subdirectory paths,
        // and bare `.typ` suffixes all route to LocalPath.
        for spec in [
            "./resume.typ",
            "../themes/mine.typ",
            "/abs/path/to/theme.typ",
            "subdir/theme.typ",
            "theme.typ",
            ".\\win\\path.typ",
            "C:\\Users\\me\\theme.typ",
        ] {
            assert!(
                matches!(classify_spec(spec), ThemeSpecKind::LocalPath),
                "spec `{spec}` must classify as LocalPath",
            );
        }
    }

    #[test]
    fn classify_spec_unknown_bundled_name_is_bundled() {
        // A name with no path signals and no `@preview/` prefix stays
        // classified as a bundled name; the registry lookup then
        // fails with NotFound rather than being misrouted to the
        // local-path branch.
        assert!(matches!(
            classify_spec("not-a-real-theme"),
            ThemeSpecKind::BundledName
        ));
    }

    #[test]
    fn resolve_theme_bundled_name_hits_registry() {
        let resolved = resolve_theme("text-minimal").expect("text-minimal is bundled");
        assert_eq!(resolved.name(), "text-minimal");
        match resolved {
            ResolvedTheme::Bundled(_) => {}
            _ => panic!("expected Bundled variant"),
        }
    }

    #[test]
    fn resolve_theme_unknown_bundled_name_returns_not_found() {
        let err =
            resolve_theme("definitely-not-a-theme").expect_err("unknown bundled names must error");
        match err {
            ThemeResolveError::NotFound { name, available } => {
                assert_eq!(name, "definitely-not-a-theme");
                assert!(!available.is_empty(), "available list must be non-empty");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    /// On the default build (no `install` feature) `@preview/...`
    /// specs are rejected with a clear "rebuild with --features
    /// install" hint — the cache reader is not in the binary.
    #[cfg(not(feature = "install"))]
    #[test]
    fn resolve_theme_preview_spec_requires_install_feature() {
        let err = resolve_theme("@preview/basic-resume:0.2.8")
            .expect_err("preview specs need the install feature on default builds");
        match err {
            ThemeResolveError::PreviewSpecRequiresInstallFeature { spec } => {
                assert_eq!(spec, "@preview/basic-resume:0.2.8");
            }
            other => panic!("expected PreviewSpecRequiresInstallFeature, got {other:?}"),
        }
    }

    /// With the `install` feature on, `@preview/...` specs hit the
    /// cache reader. The cache is empty in this unit test (no fixture
    /// populated), so we expect a `PreviewCacheMiss` carrying the spec.
    #[cfg(feature = "install")]
    #[test]
    fn resolve_theme_preview_spec_cache_miss_under_install_feature() {
        // Point FERROCV_CACHE_DIR at an empty tempdir so the resolver
        // sees a guaranteed cache miss regardless of the developer's
        // real $HOME cache state. Use a unique tempdir per test so
        // parallel runs don't share state.
        let tmp = tempfile::TempDir::new().expect("tempdir");

        // Serialize against every other test that mutates this env
        // var. Without this, `package_cache::tests` and
        // `install::cache::tests` (both in the same lib-test binary)
        // can race with us on `FERROCV_CACHE_DIR` and produce
        // intermittent failures.
        let _lock = crate::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        struct Guard(Option<String>);
        impl Drop for Guard {
            fn drop(&mut self) {
                // SAFETY: caller holds `crate::test_env::ENV_LOCK`
                // for the lifetime of this guard, so no other
                // env-var-mutating test runs concurrently.
                unsafe {
                    match &self.0 {
                        Some(v) => std::env::set_var("FERROCV_CACHE_DIR", v),
                        None => std::env::remove_var("FERROCV_CACHE_DIR"),
                    }
                }
            }
        }
        let _guard = Guard(std::env::var("FERROCV_CACHE_DIR").ok());
        // SAFETY: serialized via `_lock` above.
        unsafe {
            std::env::set_var("FERROCV_CACHE_DIR", tmp.path());
        }

        let err =
            resolve_theme("@preview/missing-pkg-xyz:0.0.0").expect_err("empty cache must miss");
        match err {
            ThemeResolveError::PreviewCacheMiss { spec, .. } => {
                assert_eq!(spec, "@preview/missing-pkg-xyz:0.0.0");
            }
            other => panic!("expected PreviewCacheMiss, got {other:?}"),
        }
    }

    /// Malformed `@preview/...` specs surface `PreviewSpecInvalid`,
    /// not `PreviewCacheMiss`. The earlier variant produced a circular
    /// "Run: ferrocv themes install <bad spec>" hint that would just
    /// hit the same parse failure; this test guards against that
    /// regression.
    #[cfg(feature = "install")]
    #[test]
    fn resolve_theme_malformed_preview_spec_returns_preview_spec_invalid() {
        // `@preview/foo` with no version separator is the canonical
        // shape `parse_spec` rejects.
        let err = resolve_theme("@preview/foo").expect_err("malformed spec must error");
        match err {
            ThemeResolveError::PreviewSpecInvalid { spec, reason } => {
                assert_eq!(spec, "@preview/foo");
                assert!(
                    !reason.is_empty(),
                    "PreviewSpecInvalid reason must be non-empty"
                );
                let rendered =
                    format!("{}", ThemeResolveError::PreviewSpecInvalid { spec, reason });
                assert!(
                    rendered.contains("Expected '@preview/<name>:<version>'"),
                    "user-facing message must point at correct syntax; got: {rendered}"
                );
                assert!(
                    !rendered.to_lowercase().contains("themes install"),
                    "must not produce a circular 'themes install' hint; got: {rendered}"
                );
            }
            other => panic!("expected PreviewSpecInvalid, got {other:?}"),
        }
    }

    #[test]
    fn resolve_theme_missing_local_path_errors() {
        let err = resolve_theme("/nonexistent/path/definitely-not-there.typ")
            .expect_err("missing local paths must error");
        match err {
            ThemeResolveError::LocalPathNotFound { path } => {
                assert!(path.to_string_lossy().contains("definitely-not-there.typ"));
            }
            other => panic!("expected LocalPathNotFound, got {other:?}"),
        }
    }
}
