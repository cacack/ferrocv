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

use std::collections::HashSet;
use std::path::PathBuf;

use super::{
    InstallError, PackageSpec,
    cache::{ensure_parent_exists, package_cache_dir},
    extract::extract_tarball,
    fetch::fetch_tarball,
    imports::scan_preview_imports,
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

/// Outcome bundle for a recursive install: the primary package's
/// outcome plus one [`InstallOutcome`] per transitive dep installed
/// or already cached during this call.
#[derive(Debug)]
pub struct InstallSummary {
    /// Outcome of installing the primary spec (the one the user typed).
    pub primary: InstallOutcome,
    /// Outcomes of every transitive dep, in the order they were
    /// resolved. Each entry pairs the spec with the install outcome
    /// (`AlreadyCached` for cache hits, `Installed` for fresh fetches).
    /// Empty if the primary had no transitive `@preview/...` imports.
    pub transitive: Vec<(PackageSpec, InstallOutcome)>,
}

/// Format a [`PackageSpec`] as the canonical `@preview/<name>:<version>`
/// string used in error messages and worklist parent labels.
fn format_spec(spec: &PackageSpec) -> String {
    format!("@preview/{}:{}", spec.name, spec.version)
}

/// Install `spec` and every `@preview/...` package it transitively
/// imports, terminating cleanly on cycles.
///
/// The primary spec is installed via [`install`]; its cache directory
/// is then scanned for `@preview/...` imports via
/// [`scan_preview_imports`], and each newly-discovered transitive is
/// installed and scanned in turn. A `(name, version)` visited set
/// guarantees termination even when packages reference each other in
/// a cycle.
///
/// On a transitive failure the error is wrapped as
/// [`InstallError::TransitiveDepFailed`] so the diagnostic preserves
/// the parent attribution for multi-level chains. The primary's cache
/// entry is left in place (a re-run won't re-fetch it) and the user
/// is expected to fix the transitive and rerun.
pub fn install_with_transitive(spec: &PackageSpec) -> Result<InstallSummary, InstallError> {
    // Install the primary first; its outcome anchors the summary.
    let primary_outcome = install(spec)?;
    let primary_label = format_spec(spec);

    // Visited set keyed on (name, version): seeded with the primary so
    // a self-referential import does not loop. Different versions of
    // the same package ARE legitimately distinct entries.
    let mut visited: HashSet<(String, String)> = HashSet::new();
    visited.insert((spec.name.clone(), spec.version.clone()));

    // Worklist entries are `(child_spec, parent_label)` so error
    // attribution survives multi-level chains: each grandchild's
    // failure names the child that imported it, not the original
    // primary.
    let mut worklist: Vec<(PackageSpec, String)> = Vec::new();

    // Seed the worklist with the primary's direct deps.
    let direct = scan_preview_imports(primary_outcome.path(), spec)?;
    for child in direct {
        let key = (child.name.clone(), child.version.clone());
        if visited.contains(&key) {
            continue;
        }
        worklist.push((child, primary_label.clone()));
    }

    let mut transitive: Vec<(PackageSpec, InstallOutcome)> = Vec::new();

    while let Some((child_spec, parent_label)) = worklist.pop() {
        let key = (child_spec.name.clone(), child_spec.version.clone());
        if !visited.insert(key) {
            // Already installed (or attempted) in this call.
            continue;
        }
        let child_label = format_spec(&child_spec);
        let child_outcome =
            install(&child_spec).map_err(|err| InstallError::TransitiveDepFailed {
                parent: parent_label.clone(),
                child: child_label.clone(),
                source: Box::new(err),
            })?;
        // Even on `AlreadyCached`, scan for the dep's own deps —
        // a previously-cached package may have transitives the
        // current cache hasn't fetched yet. The visited set
        // prevents re-scanning the same package twice in this call.
        let grandchildren = scan_preview_imports(child_outcome.path(), &child_spec)?;
        transitive.push((child_spec, child_outcome));
        for grandchild in grandchildren {
            let key = (grandchild.name.clone(), grandchild.version.clone());
            if visited.contains(&key) {
                continue;
            }
            worklist.push((grandchild, child_label.clone()));
        }
    }

    Ok(InstallSummary {
        primary: primary_outcome,
        transitive,
    })
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

    use crate::test_env::ENV_LOCK;
    use std::path::Path;

    /// Snapshot+restore guard for `FERROCV_CACHE_DIR`. Mirrors the
    /// shape used in `crate::install::cache::tests` and
    /// `crate::package_cache::tests` so a panicking test body still
    /// leaves the env intact.
    struct EnvGuard {
        prior: Option<String>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: tests are serialized via ENV_LOCK held by the
            // caller of `with_cache_dir`.
            unsafe {
                match &self.prior {
                    Some(v) => std::env::set_var("FERROCV_CACHE_DIR", v),
                    None => std::env::remove_var("FERROCV_CACHE_DIR"),
                }
            }
        }
    }

    fn with_cache_dir<F: FnOnce()>(value: &Path, body: F) {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _guard = EnvGuard {
            prior: std::env::var("FERROCV_CACHE_DIR").ok(),
        };
        // SAFETY: serialized via ENV_LOCK above.
        unsafe {
            std::env::set_var("FERROCV_CACHE_DIR", value);
        }
        body();
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir -p");
        }
        std::fs::write(path, content).expect("write fixture");
    }

    /// Pre-populate a cache entry so `install()` returns
    /// `AlreadyCached` instead of attempting a network fetch. `lib_typ`
    /// is the contents of `src/lib.typ`; pass an `@preview/...` import
    /// to seed transitive-dep discovery.
    fn populate_cache_entry(cache_root: &Path, name: &str, version: &str, lib_typ: &str) {
        let pkg = cache_root
            .join("packages")
            .join("preview")
            .join(name)
            .join(version);
        write_file(
            &pkg.join("typst.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"src/lib.typ\"\n",
            ),
        );
        write_file(&pkg.join("src/lib.typ"), lib_typ);
    }

    #[test]
    fn install_summary_default_when_no_transitives() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Primary's source has no @preview/... imports.
        populate_cache_entry(tmp.path(), "leaf-pkg", "1.0.0", "= Hello\n");
        with_cache_dir(tmp.path(), || {
            let spec = PackageSpec {
                namespace: "preview".to_owned(),
                name: "leaf-pkg".to_owned(),
                version: "1.0.0".to_owned(),
            };
            let summary =
                install_with_transitive(&spec).expect("populated cache must short-circuit");
            assert!(
                matches!(summary.primary, InstallOutcome::AlreadyCached { .. }),
                "primary must be AlreadyCached: {:?}",
                summary.primary,
            );
            assert!(
                summary.transitive.is_empty(),
                "no @preview imports => empty transitive list: {:?}",
                summary.transitive,
            );
        });
    }

    #[test]
    fn install_summary_visits_each_dep_once() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Cycle: A imports B, B imports A. Both pre-populated so
        // `install()` returns AlreadyCached for each. The recursive
        // driver must terminate via the visited set rather than
        // looping forever.
        populate_cache_entry(
            tmp.path(),
            "alpha",
            "1.0.0",
            "#import \"@preview/beta:1.0.0\": *\n",
        );
        populate_cache_entry(
            tmp.path(),
            "beta",
            "1.0.0",
            "#import \"@preview/alpha:1.0.0\": *\n",
        );
        with_cache_dir(tmp.path(), || {
            let primary = PackageSpec {
                namespace: "preview".to_owned(),
                name: "alpha".to_owned(),
                version: "1.0.0".to_owned(),
            };
            let summary =
                install_with_transitive(&primary).expect("cycle must terminate via visited set");
            assert!(matches!(
                summary.primary,
                InstallOutcome::AlreadyCached { .. }
            ));
            // Primary itself is NOT in the transitive list.
            assert_eq!(
                summary.transitive.len(),
                1,
                "cycle must yield exactly one transitive entry; got {:?}",
                summary
                    .transitive
                    .iter()
                    .map(|(s, _)| (&s.name, &s.version))
                    .collect::<Vec<_>>(),
            );
            let (child_spec, child_outcome) = &summary.transitive[0];
            assert_eq!(child_spec.name, "beta");
            assert_eq!(child_spec.version, "1.0.0");
            assert!(
                matches!(child_outcome, InstallOutcome::AlreadyCached { .. }),
                "child must be AlreadyCached: {child_outcome:?}",
            );
        });
    }

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
