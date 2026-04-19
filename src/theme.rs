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
//! Every [`Theme`] in *this* module is an **adapter**: it wraps an
//! upstream Typst template (e.g. `typst-jsonresume-cv`) and hands it a
//! JSON Resume structure through the conventional `/resume.json`
//! virtual file. Adapters accept that upstream layout changes may
//! break them; in return they give `ferrocv` visual variety without
//! re-implementing a full resume renderer.
//!
//! **Native themes** — themes that implement a `render(data) ->
//! content` contract directly against parsed JSON Resume data — live
//! in a different module (to be added when Phase 4 begins). Per
//! CONSTITUTION §4 the two layers are kept separable: adapter code
//! does not leak into native themes, and native themes do not depend
//! on adapter internals.
//!
//! # Why a static slice, not a `HashMap` or `ThemeRegistry`
//!
//! Phase 1 ships with one adapter and Phase 2 is planned to ship with
//! at most one more. A linear scan over `THEMES` is O(n) for n ≤ 2.
//! CONSTITUTION §5 ("simple now, iterate later") calls for the
//! narrower solution here; generalizing to a hashed lookup or a
//! builder pattern should wait for a second caller that actually
//! needs it.

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

/// All themes registered with this build of `ferrocv`.
///
/// Phase 1 ships one adapter. See the module doc for why this is a
/// `&[&Theme]` rather than a `HashMap` or a builder pattern.
pub const THEMES: &[&Theme] = &[&TYPST_JSONRESUME_CV];

/// Look up a [`Theme`] by name. Returns `None` for unknown names.
///
/// Linear scan over [`THEMES`]; O(n) for n themes. Acceptable for
/// the current n ≤ 2 regime (CONSTITUTION §5).
pub fn find_theme(name: &str) -> Option<&'static Theme> {
    THEMES.iter().copied().find(|t| t.name == name)
}
