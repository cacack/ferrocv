//! Read a Typst Universe `@preview/...` package out of the local
//! installer cache and materialize it as an [`OwnedTheme`] that the
//! render pipeline can compile.
//!
//! Stage C of issue #41 connects the offline cache populated by
//! Stage B's `ferrocv themes install` to the render path. The reader
//! is **filesystem-only** — no `ureq`, no `flate2`, no `tar`. It
//! shares the cache-path resolver and the manifest parser with
//! [`crate::install`] (gated behind the same `install` Cargo feature)
//! so we have one source of truth for the on-disk shape.
//!
//! # Architectural boundary (CONSTITUTION §6.1)
//!
//! This module is gated behind the `install` Cargo feature, just like
//! [`crate::install`], because both depend on the `toml` and `dirs`
//! crates. Crucially, the resolver entry point in [`crate::cli`]
//! decides on a cache-miss to surface a clear "run `ferrocv themes
//! install`" error rather than calling into [`crate::install`]: the
//! render path does not import the network-capable installer module
//! even when the feature is on. The "no network at render time"
//! guarantee is a code-boundary property, not just a feature-flag
//! property.

use std::path::{Path, PathBuf};

use crate::install::cache::package_cache_dir;
use crate::install::manifest::parse_manifest;
use crate::install::spec::PackageSpec;
use crate::theme::{OwnedTheme, ThemeResolveError};

/// Read every theme file in a cached package's directory tree and
/// assemble an [`OwnedTheme`] anchored at the package's manifest
/// `entrypoint`.
///
/// Behavior:
///
/// 1. Resolve the cache directory for `(name, version)` via
///    [`package_cache_dir`].
/// 2. If the directory does not exist, return
///    [`ThemeResolveError::PreviewCacheMiss`] carrying the missing
///    path so the CLI can format the "run `ferrocv themes install`"
///    hint without re-deriving the location.
/// 3. Read `<dir>/typst.toml`, parse the minimal manifest, validate
///    that the manifest's `entrypoint` is a relative path that stays
///    inside the package directory.
/// 4. Walk the package tree, reading every regular file with a
///    suffix Typst can compile (`.typ`) into the [`OwnedTheme`]'s
///    `files` vector. Symlinks are not followed; non-`.typ` files
///    (`README.md`, `LICENSE`, `CHANGELOG.md`, etc.) are skipped.
///    The manifest's `typst.toml` is also skipped — Typst does not
///    read it at compile time.
/// 5. Return the [`OwnedTheme`] keyed under the virtual-path prefix
///    `/themes/preview/<name>/<version>/...`. The version is part
///    of the prefix so two cached versions of the same package
///    cannot collide in the World.
///
/// # Offline guarantee
///
/// This function performs zero network calls. Cache misses produce
/// [`ThemeResolveError::PreviewCacheMiss`]; the CLI handles that by
/// pointing the user at `ferrocv themes install` rather than calling
/// the installer transitively. Render and validate stay offline.
pub fn resolve_preview_spec_from_cache(
    spec: &PackageSpec,
) -> Result<OwnedTheme, ThemeResolveError> {
    let raw_spec = format!("@preview/{}:{}", spec.name, spec.version);

    // Errors from `package_cache_dir` mean we could not even
    // determine where the cache lives (e.g. `FERROCV_CACHE_DIR=""`).
    // Surface those as a cache-miss too so the CLI message stays
    // single-shape; the formatted path will read e.g. `<unresolved>`.
    let cache_dir = package_cache_dir(&spec.name, &spec.version).map_err(|err| {
        ThemeResolveError::PreviewCacheMiss {
            spec: raw_spec.clone(),
            expected_path: PathBuf::from(format!("<unresolved: {err}>")),
        }
    })?;

    if !cache_dir.is_dir() {
        return Err(ThemeResolveError::PreviewCacheMiss {
            spec: raw_spec,
            expected_path: cache_dir,
        });
    }

    let manifest_path = cache_dir.join("typst.toml");
    let manifest_src = std::fs::read_to_string(&manifest_path).map_err(|source| {
        ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec.clone(),
            path: manifest_path.clone(),
            reason: format!("could not read typst.toml: {source}"),
        }
    })?;
    let manifest =
        parse_manifest(&manifest_src).map_err(|err| ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec.clone(),
            path: manifest_path.clone(),
            reason: format!("typst.toml parse failed: {err}"),
        })?;
    // The on-disk manifest must agree with the spec the user typed —
    // catches a misnamed cache entry or a `--theme` typo that pointed
    // at the wrong cached package by accident.
    if manifest.name != spec.name || manifest.version != spec.version {
        return Err(ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec,
            path: manifest_path,
            reason: format!(
                "manifest declares @preview/{}:{} but cache directory is for @preview/{}:{}",
                manifest.name, manifest.version, spec.name, spec.version,
            ),
        });
    }

    // Parse-time validation in `parse_manifest` already rejected
    // `..` segments, absolute paths, and Windows drive prefixes, so
    // joining is safe here. The resulting absolute path is used only
    // for diagnostic output below.
    let entrypoint_abs = cache_dir.join(&manifest.entrypoint);
    if !entrypoint_abs.is_file() {
        return Err(ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec,
            path: entrypoint_abs,
            reason: "manifest entrypoint does not exist on disk".to_owned(),
        });
    }

    let virtual_prefix = format!("/themes/preview/{}/{}", spec.name, spec.version);
    let entrypoint_virtual = format!(
        "{virtual_prefix}/{}",
        // Normalize back-slashes to forward-slashes so the virtual
        // path is platform-independent (Typst's VirtualPath is
        // forward-slash-only). The manifest validator already
        // rejected `..` segments, so the only normalization left is
        // the separator flip.
        manifest.entrypoint.replace('\\', "/"),
    );

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    collect_typ_files(
        &cache_dir,
        &cache_dir,
        &virtual_prefix,
        &mut files,
        &raw_spec,
    )?;

    // Sanity check: the entrypoint must appear among the files we
    // collected, otherwise `FerrocvWorld::from_bundle` will panic.
    if !files.iter().any(|(p, _)| p == &entrypoint_virtual) {
        return Err(ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec,
            path: cache_dir,
            reason: format!(
                "manifest entrypoint {} did not match any cached .typ file under {}",
                manifest.entrypoint, virtual_prefix,
            ),
        });
    }

    Ok(OwnedTheme {
        name: format!("@preview/{}:{}", spec.name, spec.version),
        files,
        entrypoint: entrypoint_virtual,
    })
}

/// Walk a cached-package directory tree and push every `.typ` file
/// into `out` keyed under `<virtual_prefix>/<relative-path>`.
///
/// Symlinks are not followed (`metadata.is_file()` follows symlinks
/// but we additionally check `symlink_metadata().is_symlink()` and
/// skip those). Non-`.typ` files are ignored — Typst will not read
/// `README.md`, `LICENSE`, etc. at compile time, and they are not
/// theme assets we need to register in the World.
fn collect_typ_files(
    root: &Path,
    dir: &Path,
    virtual_prefix: &str,
    out: &mut Vec<(String, Vec<u8>)>,
    raw_spec: &str,
) -> Result<(), ThemeResolveError> {
    let entries =
        std::fs::read_dir(dir).map_err(|source| ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec.to_owned(),
            path: dir.to_path_buf(),
            reason: format!("read_dir failed: {source}"),
        })?;
    for entry in entries {
        let entry = entry.map_err(|source| ThemeResolveError::PreviewCacheCorrupt {
            spec: raw_spec.to_owned(),
            path: dir.to_path_buf(),
            reason: format!("dir entry read failed: {source}"),
        })?;
        let path = entry.path();
        // Reject symlinks defensively. `tar::Archive::unpack` does
        // not normally extract symlinks, but a corrupted cache or a
        // local edit could introduce one; following it would let a
        // resume-time read escape the cache root.
        let symlink_meta =
            entry
                .file_type()
                .map_err(|source| ThemeResolveError::PreviewCacheCorrupt {
                    spec: raw_spec.to_owned(),
                    path: path.clone(),
                    reason: format!("file type read failed: {source}"),
                })?;
        if symlink_meta.is_symlink() {
            // Skip silently — non-fatal, matches "if Typst can't see
            // it, neither will the resume render" semantics.
            continue;
        }
        if symlink_meta.is_dir() {
            collect_typ_files(root, &path, virtual_prefix, out, raw_spec)?;
            continue;
        }
        if !symlink_meta.is_file() {
            continue;
        }
        // Only ingest `.typ` source files. Everything else
        // (README, LICENSE, manifest, etc.) is not part of the
        // theme bundle Typst compiles against.
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if !ext.eq_ignore_ascii_case("typ") {
            continue;
        }
        let relative =
            path.strip_prefix(root)
                .map_err(|_| ThemeResolveError::PreviewCacheCorrupt {
                    spec: raw_spec.to_owned(),
                    path: path.clone(),
                    reason: "cached file path escaped cache root".to_owned(),
                })?;
        // Build a forward-slash virtual path. `Path::display()` on
        // Windows uses back-slashes; we explicitly join components so
        // the result is portable.
        let mut virtual_path = String::from(virtual_prefix);
        for component in relative.components() {
            match component {
                std::path::Component::Normal(name) => {
                    virtual_path.push('/');
                    let s =
                        name.to_str()
                            .ok_or_else(|| ThemeResolveError::PreviewCacheCorrupt {
                                spec: raw_spec.to_owned(),
                                path: path.clone(),
                                reason: "cached file path is not valid UTF-8".to_owned(),
                            })?;
                    virtual_path.push_str(s);
                }
                // Skip CurDir; ParentDir / RootDir / Prefix should
                // be unreachable because we built `relative` via
                // `strip_prefix`, but we handle them defensively.
                std::path::Component::CurDir => {}
                _ => {
                    return Err(ThemeResolveError::PreviewCacheCorrupt {
                        spec: raw_spec.to_owned(),
                        path: path.clone(),
                        reason: "unexpected component in relative cache path".to_owned(),
                    });
                }
            }
        }
        let bytes =
            std::fs::read(&path).map_err(|source| ThemeResolveError::PreviewCacheCorrupt {
                spec: raw_spec.to_owned(),
                path: path.clone(),
                reason: format!("read failed: {source}"),
            })?;
        out.push((virtual_path, bytes));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Snapshot+restore guard for `FERROCV_CACHE_DIR` so a panicking
    /// test body still leaves the env intact.
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

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir -p");
        }
        std::fs::write(path, content).expect("write fixture");
    }

    /// Construct a minimal valid cached package under `cache_root`.
    fn populate_minimal_package(cache_root: &Path, name: &str, version: &str) -> PathBuf {
        let pkg = cache_root
            .join("packages")
            .join("preview")
            .join(name)
            .join(version);
        write(
            &pkg.join("typst.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"src/lib.typ\"\n",
            ),
        );
        write(&pkg.join("src/lib.typ"), "= Hello\n");
        write(&pkg.join("README.md"), "ignored\n");
        pkg
    }

    #[test]
    fn cache_miss_returns_preview_cache_miss_with_expected_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        with_cache_dir(tmp.path(), || {
            let spec = PackageSpec {
                namespace: "preview".to_owned(),
                name: "missing-pkg".to_owned(),
                version: "1.0.0".to_owned(),
            };
            let err =
                resolve_preview_spec_from_cache(&spec).expect_err("missing cache entry must error");
            match err {
                ThemeResolveError::PreviewCacheMiss {
                    spec,
                    expected_path,
                } => {
                    assert_eq!(spec, "@preview/missing-pkg:1.0.0");
                    assert!(
                        expected_path
                            .to_string_lossy()
                            .contains("missing-pkg/1.0.0"),
                        "expected path must point at missing-pkg/1.0.0; got: {}",
                        expected_path.display(),
                    );
                }
                other => panic!("expected PreviewCacheMiss, got {other:?}"),
            }
        });
    }

    #[test]
    fn cache_hit_assembles_owned_theme() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _pkg = populate_minimal_package(tmp.path(), "demo-pkg", "0.1.0");
        with_cache_dir(tmp.path(), || {
            let spec = PackageSpec {
                namespace: "preview".to_owned(),
                name: "demo-pkg".to_owned(),
                version: "0.1.0".to_owned(),
            };
            let theme =
                resolve_preview_spec_from_cache(&spec).expect("populated cache must resolve");
            assert_eq!(theme.name, "@preview/demo-pkg:0.1.0");
            assert_eq!(
                theme.entrypoint,
                "/themes/preview/demo-pkg/0.1.0/src/lib.typ"
            );
            // Non-`.typ` files are skipped: README.md must not appear
            // in the file list.
            assert!(
                theme.files.iter().any(|(p, _)| p.ends_with("/src/lib.typ")),
                "lib.typ must appear in files; got {:?}",
                theme.files.iter().map(|(p, _)| p).collect::<Vec<_>>(),
            );
            assert!(
                theme
                    .files
                    .iter()
                    .all(|(p, _)| !p.to_lowercase().ends_with(".md")),
                "README.md must be filtered out",
            );
        });
    }

    #[test]
    fn cache_hit_with_manifest_mismatch_is_corrupt() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Manifest declares a different name than the directory.
        let pkg = tmp.path().join("packages/preview/asked-for/1.0.0");
        write(
            &pkg.join("typst.toml"),
            "[package]\nname = \"actually-different\"\nversion = \"1.0.0\"\nentrypoint = \"src/lib.typ\"\n",
        );
        write(&pkg.join("src/lib.typ"), "= Hi\n");
        with_cache_dir(tmp.path(), || {
            let spec = PackageSpec {
                namespace: "preview".to_owned(),
                name: "asked-for".to_owned(),
                version: "1.0.0".to_owned(),
            };
            let err =
                resolve_preview_spec_from_cache(&spec).expect_err("manifest mismatch must error");
            assert!(
                matches!(err, ThemeResolveError::PreviewCacheCorrupt { .. }),
                "expected PreviewCacheCorrupt; got: {err:?}",
            );
        });
    }
}
