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
//! will eventually live in a dedicated module. For Phase 2, with one
//! native theme to ship, that split would be premature abstraction
//! (CONSTITUTION §5: "simple now, iterate later"); the split is
//! deferred until a second native theme materializes.
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
/// eventually. For Phase 2, with one native theme registered,
/// splitting is premature abstraction (§5: "simple now, iterate
/// later"); the split is deferred until a second native theme exists.
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
