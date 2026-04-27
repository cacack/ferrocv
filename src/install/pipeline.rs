//! `install()` orchestrator — the public entry point for fetching and
//! caching a Typst Universe package.
//!
//! Glues [`super::fetch`], [`super::extract`], [`super::manifest`],
//! and [`super::cache`] into one idempotent operation:
//!
//! 1. Resolve the final cache path.
//! 2. If it already exists, return it (cache hit).
//! 3. Else: create a staging `TempDir` alongside the final path,
//!    fetch the tarball, extract into the staging dir, parse the
//!    manifest, verify name/version match the spec.
//! 4. Atomically `fs::rename` the staging dir onto the final path.
//!    On rename-loses-race (another concurrent install won), clean up
//!    the staging dir and return the winner's path.

use std::path::PathBuf;

use super::{
    InstallError, PackageSpec,
    cache::{ensure_parent_exists, package_cache_dir},
    extract::extract_tarball,
    fetch::fetch_tarball,
    manifest::parse_manifest,
};

/// Outcome of [`install`] — either a cache hit (nothing fetched) or a
/// fresh install (tarball fetched and extracted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Package was already in the cache; no network fetch happened.
    AlreadyCached {
        /// Absolute path to the cached package directory.
        path: PathBuf,
    },
    /// Package was fetched from the registry and written to the cache.
    Installed {
        /// Absolute path to the cached package directory.
        path: PathBuf,
    },
}

impl InstallOutcome {
    /// Borrow the cached package path regardless of outcome.
    pub fn path(&self) -> &PathBuf {
        match self {
            InstallOutcome::AlreadyCached { path } | InstallOutcome::Installed { path } => path,
        }
    }
}

/// Fetch + extract + cache a Typst Universe package.
///
/// Idempotent: if the cache directory already exists, returns
/// [`InstallOutcome::AlreadyCached`] without making a network call.
/// Otherwise fetches the tarball over HTTPS, extracts into a staging
/// temp dir, verifies the manifest matches the spec, and atomically
/// renames the staging dir onto the final cache path.
pub fn install(spec: &PackageSpec) -> Result<InstallOutcome, InstallError> {
    let final_dir = package_cache_dir(&spec.name, &spec.version)?;
    if final_dir.is_dir() {
        return Ok(InstallOutcome::AlreadyCached { path: final_dir });
    }

    let parent = ensure_parent_exists(&final_dir)?;
    let temp = tempfile::TempDir::new_in(&parent).map_err(|source| InstallError::Io {
        context: format!("create staging temp dir under {}", parent.display()),
        source,
    })?;

    let bytes = fetch_tarball(spec)?;
    extract_tarball(&bytes, temp.path())?;
    verify_manifest(spec, temp.path())?;

    // Atomic publish: rename temp dir onto the final path. On POSIX
    // this is truly atomic; on Windows it is best-effort atomic when
    // source and destination share a filesystem (guaranteed here
    // because we anchored the TempDir under `parent`).
    let staged = temp.keep();
    match std::fs::rename(&staged, &final_dir) {
        Ok(()) => Ok(InstallOutcome::Installed { path: final_dir }),
        Err(_) if final_dir.is_dir() => {
            // Concurrent install won the race; our copy is redundant.
            let _ = std::fs::remove_dir_all(&staged);
            Ok(InstallOutcome::AlreadyCached { path: final_dir })
        }
        Err(source) => {
            let _ = std::fs::remove_dir_all(&staged);
            Err(InstallError::Io {
                context: format!("publish cache entry {}", final_dir.display()),
                source,
            })
        }
    }
}

/// Read the staged `typst.toml` and assert its name/version match
/// `spec`. Returns [`InstallError::ManifestMissing`] if the file is
/// absent, [`InstallError::ManifestParse`] if it is malformed, or
/// [`InstallError::ManifestMismatch`] if the declared name/version
/// does not match the spec we asked for.
pub(crate) fn verify_manifest(
    spec: &PackageSpec,
    staged_root: &std::path::Path,
) -> Result<(), InstallError> {
    let manifest_path = staged_root.join("typst.toml");
    if !manifest_path.is_file() {
        return Err(InstallError::ManifestMissing {
            expected: manifest_path,
        });
    }
    let manifest_src =
        std::fs::read_to_string(&manifest_path).map_err(|source| InstallError::Io {
            context: format!("read {}", manifest_path.display()),
            source,
        })?;
    let manifest = parse_manifest(&manifest_src)?;
    if manifest.name != spec.name || manifest.version != spec.version {
        return Err(InstallError::ManifestMismatch {
            expected: format!("@preview/{}:{}", spec.name, spec.version),
            found: format!("@preview/{}:{}", manifest.name, manifest.version),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_manifest_accepts_matching_tarball() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            temp.path().join("typst.toml"),
            r#"
[package]
name = "basic-resume"
version = "0.2.8"
entrypoint = "src/lib.typ"
"#,
        )
        .unwrap();
        let spec = PackageSpec {
            namespace: "preview".to_owned(),
            name: "basic-resume".to_owned(),
            version: "0.2.8".to_owned(),
        };
        verify_manifest(&spec, temp.path()).expect("matching manifest passes");
    }

    #[test]
    fn verify_manifest_rejects_name_mismatch() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            temp.path().join("typst.toml"),
            r#"
[package]
name = "different-name"
version = "0.2.8"
entrypoint = "src/lib.typ"
"#,
        )
        .unwrap();
        let spec = PackageSpec {
            namespace: "preview".to_owned(),
            name: "basic-resume".to_owned(),
            version: "0.2.8".to_owned(),
        };
        let err = verify_manifest(&spec, temp.path()).expect_err("name mismatch must fail");
        match err {
            InstallError::ManifestMismatch { expected, found } => {
                assert!(expected.contains("basic-resume"));
                assert!(found.contains("different-name"));
            }
            other => panic!("expected ManifestMismatch, got {other:?}"),
        }
    }

    #[test]
    fn verify_manifest_rejects_version_mismatch() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            temp.path().join("typst.toml"),
            r#"
[package]
name = "basic-resume"
version = "9.9.9"
entrypoint = "src/lib.typ"
"#,
        )
        .unwrap();
        let spec = PackageSpec {
            namespace: "preview".to_owned(),
            name: "basic-resume".to_owned(),
            version: "0.2.8".to_owned(),
        };
        let err = verify_manifest(&spec, temp.path()).expect_err("version mismatch must fail");
        assert!(matches!(err, InstallError::ManifestMismatch { .. }));
    }

    #[test]
    fn verify_manifest_rejects_missing_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let spec = PackageSpec {
            namespace: "preview".to_owned(),
            name: "basic-resume".to_owned(),
            version: "0.2.8".to_owned(),
        };
        let err = verify_manifest(&spec, temp.path()).expect_err("missing manifest must fail");
        assert!(matches!(err, InstallError::ManifestMissing { .. }));
    }
}
