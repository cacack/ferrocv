//! `ferrocv theme install` support — the single, enumerated
//! network-permitted entry point per CONSTITUTION.md §6.1
//! (post-Stage-B amendment).
//!
//! The entire module tree is gated behind the `install` Cargo feature.
//! With the feature off (the default build), none of the contents of
//! this module compile, no network-capable crate
//! (`ureq`/`flate2`/`tar`/`dirs`) enters the dependency graph, and the
//! `render` and `validate` call graphs stay network-free by
//! construction.
//!
//! # Architecture
//!
//! The installer is a small orchestrated pipeline:
//!
//! 1. [`spec::parse_spec`] parses `@preview/<name>:<version>` strings
//!    into a [`spec::PackageSpec`].
//! 2. [`cache::package_cache_dir`] resolves the filesystem location
//!    that cached package should live at, honoring `FERROCV_CACHE_DIR`
//!    for tests and power users, else `dirs::cache_dir()`.
//! 3. [`fetch::fetch_tarball`] pulls the `.tar.gz` over HTTPS via
//!    `ureq` (rustls + ring).
//! 4. [`extract::extract_tarball`] unpacks the bytes into a staging
//!    temp directory via `flate2::read::GzDecoder` + `tar::Archive`.
//! 5. [`manifest::parse_manifest`] reads the staged `typst.toml` and
//!    asserts the name/version match the requested spec (v1 integrity
//!    = TLS only + manifest match; no checksum or signature
//!    verification because the registry does not publish them).
//! 6. [`pipeline::install`] glues these together and atomically
//!    renames the staged temp dir onto the final cache path. Concurrent
//!    invocations are race-safe by construction: the loser cleans up
//!    and returns the winner's path.
//!
//! Each stage is a separate module so failures have named owners in
//! [`InstallError`] diagnostics and so Stage C can reuse
//! [`cache`] and [`manifest`] directly without pulling in
//! [`fetch`]'s `ureq` graph.

pub mod cache;
pub mod extract;
pub mod fetch;
pub mod manifest;
pub mod pipeline;
pub mod spec;

pub use pipeline::install;
pub use spec::PackageSpec;

use std::fmt;
use std::path::PathBuf;

/// Errors returned by every step of the install pipeline.
///
/// All variants carry enough context for a single-line `error: ...`
/// stderr message; the CLI maps every variant to exit code 2.
/// Constructing an [`InstallError`] is always cheap — large payloads
/// (tarball bytes, IO buffers) are consumed earlier in the pipeline
/// and never embedded here.
#[derive(Debug)]
pub enum InstallError {
    /// The raw `--<spec>` string did not parse as
    /// `@preview/<name>:<version>`. Carries the user-typed input
    /// verbatim so the diagnostic can echo it back.
    InvalidSpec {
        /// The raw spec string the user passed.
        raw: String,
        /// Human-readable reason the spec was rejected.
        reason: String,
    },
    /// HTTPS GET failed before we got a status line. Carries the URL
    /// we tried so the diagnostic is actionable.
    Http {
        /// The tarball URL we tried to fetch.
        url: String,
        /// The underlying transport error message.
        reason: String,
    },
    /// HTTPS GET returned a non-success status code (4xx / 5xx).
    /// 404 is the common case: the package/version does not exist in
    /// the registry.
    HttpStatus {
        /// The tarball URL we tried to fetch.
        url: String,
        /// The HTTP status code returned by the registry.
        status: u16,
    },
    /// An IO operation failed during install — creating the cache
    /// directory, writing to the staging temp dir, reading a staged
    /// manifest, or renaming the staged dir onto its final path.
    Io {
        /// Short human-readable context (e.g. "create cache dir").
        context: String,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// Tarball extraction failed — malformed gzip, a malformed tar
    /// entry, or a path-traversal attempt. `tar::Archive::unpack`
    /// normalizes paths by default, so extraction failures here are
    /// typically real corruption rather than benign skips.
    Extract {
        /// Short human-readable context (e.g. "extract tarball").
        context: String,
        /// The underlying IO error from the tar/gzip layer.
        source: std::io::Error,
    },
    /// A tarball was extracted successfully but its `typst.toml` was
    /// missing. The registry requires every package to ship a
    /// manifest at the archive root; this means the tarball is
    /// malformed.
    ManifestMissing {
        /// Path we expected to find `typst.toml` at.
        expected: PathBuf,
    },
    /// `typst.toml` was found but failed to parse as valid TOML or
    /// was missing one of the required `package.name`,
    /// `package.version`, `package.entrypoint` fields.
    ManifestParse {
        /// Short human-readable detail from the parser.
        reason: String,
    },
    /// The tarball's `typst.toml` declared a package name or version
    /// that does not match the spec we asked for. This is our only
    /// integrity signal beyond TLS for v1 — it catches a tarball
    /// served at the wrong URL (registry bug) or a name/version typo
    /// in a `typst.toml` we pulled down.
    ManifestMismatch {
        /// What the spec asked for, `@preview/<name>:<version>`.
        expected: String,
        /// What the manifest actually said.
        found: String,
    },
    /// The user's platform does not have a resolvable cache directory
    /// and `FERROCV_CACHE_DIR` was unset. Rare — `dirs::cache_dir()`
    /// returns `Some` on every common platform.
    CacheDirUnresolved,
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallError::InvalidSpec { raw, reason } => {
                write!(
                    f,
                    "invalid package spec `{raw}`: {reason} \
                     (expected format: @preview/<name>:<version>)"
                )
            }
            InstallError::Http { url, reason } => {
                write!(f, "failed to fetch {url}: {reason}")
            }
            InstallError::HttpStatus { url, status } => {
                write!(
                    f,
                    "registry returned HTTP {status} for {url} \
                     (check the package name and version at \
                     https://typst.app/universe/)"
                )
            }
            InstallError::Io { context, source } => {
                write!(f, "{context}: {source}")
            }
            InstallError::Extract { context, source } => {
                write!(f, "{context}: {source}")
            }
            InstallError::ManifestMissing { expected } => {
                write!(
                    f,
                    "package tarball is missing typst.toml at {}",
                    expected.display(),
                )
            }
            InstallError::ManifestParse { reason } => {
                write!(f, "failed to parse typst.toml: {reason}")
            }
            InstallError::ManifestMismatch { expected, found } => {
                write!(
                    f,
                    "manifest mismatch: asked for {expected}, tarball declared {found}",
                )
            }
            InstallError::CacheDirUnresolved => {
                write!(
                    f,
                    "could not determine the user cache directory; \
                     set FERROCV_CACHE_DIR to an explicit path and retry",
                )
            }
        }
    }
}

impl std::error::Error for InstallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InstallError::Io { source, .. } | InstallError::Extract { source, .. } => Some(source),
            _ => None,
        }
    }
}
