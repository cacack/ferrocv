//! Scenario-style black-box tests for `ferrocv render --theme @preview/...`.
//!
//! Entire file is gated behind `#[cfg(feature = "install")]` because
//! the cache reader (`src/package_cache.rs`) and the cache-path
//! helpers (`src/install/cache.rs`) live behind that feature flag.
//! The default-features path (where `--theme @preview/...` errors with
//! a "rebuild with --features install" hint) is covered in
//! `tests/render_cli.rs::render_with_preview_spec_*`.
//!
//! Coverage:
//! - Cache hit: a pre-extracted fixture under
//!   `tests/fixtures/cached-preview/basic-resume/0.2.8/` is staged into
//!   a tempdir-backed cache, then `ferrocv render --theme @preview/basic-resume:0.2.8`
//!   compiles the package against the standard test resume.
//! - Cache miss: with an empty tempdir as the cache, the same render
//!   exits 2 with a stderr message pointing at `ferrocv themes install`.
//! - Inline-import regression: a cached package whose Typst source
//!   does `#import "@preview/cetz:0.2.0": *"` still fails to compile,
//!   proving CONSTITUTION §6.1 inline-import rejection survives
//!   Stage C's pre-`World` resolver.

#![cfg(feature = "install")]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

/// Absolute path to a JSON fixture under `tests/fixtures/`.
fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{name}.json"));
    path
}

/// Build a `Command` for the `ferrocv` binary under test.
fn ferrocv() -> Command {
    Command::cargo_bin("ferrocv").expect("binary `ferrocv` must be built with --features install")
}

/// Recursively copy `src` into `dst`, creating `dst` if it does not
/// exist. Plain files and directories are copied; anything else
/// (symlinks, devices, FIFOs) trips the explicit `else` arm and
/// panics. Fixture trees do not contain such entries today, but if a
/// regression introduces one we want the test to fail loudly rather
/// than silently exclude it (the `is_dir`/`is_file` checks both return
/// `false` for symlinks, so without the explicit panic arm a symlinked
/// file would be silently dropped from the staged cache).
fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("mkdir destination");
    for entry in std::fs::read_dir(src).expect("read_dir") {
        let entry = entry.expect("dir entry");
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type().expect("file type");
        if ft.is_dir() {
            copy_dir_recursive(&from, &to);
        } else if ft.is_file() {
            std::fs::copy(&from, &to).expect("copy file");
        } else {
            panic!(
                "unexpected non-file/non-dir entry in fixture: {} (file_type: {:?})",
                from.display(),
                ft,
            );
        }
    }
}

/// Lay a fixture cached package under `<cache_root>/packages/preview/<name>/<version>/`.
fn stage_cached_package(cache_root: &Path, fixture_root: &Path, name: &str, version: &str) {
    let dest = cache_root
        .join("packages")
        .join("preview")
        .join(name)
        .join(version);
    copy_dir_recursive(fixture_root, &dest);
}

/// Path to the in-tree `cached-preview/<name>/<version>/` fixture
/// directory that mirrors what Stage B's installer would have written.
fn cached_preview_fixture(name: &str, version: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("cached-preview");
    path.push(name);
    path.push(version);
    path
}

/// Path to the in-tree malicious-import fixture used by the §6.1
/// inline-import regression.
fn malicious_preview_fixture(name: &str, version: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("cached-preview-malicious");
    path.push(name);
    path.push(version);
    path
}

/// Cache hit: the pre-staged fixture is found, parsed, and compiled
/// against the standard `render_full.json` resume. The `--format text`
/// path is deterministic, so we extract text and assert the resume
/// name round-tripped through Typst — proves the cached package's
/// `lib.typ` ran and read `/resume.json` correctly.
#[test]
fn render_resolves_preview_from_cache() {
    let cache = tempfile::TempDir::new().expect("tempdir cache");
    stage_cached_package(
        cache.path(),
        &cached_preview_fixture("basic-resume", "0.2.8"),
        "basic-resume",
        "0.2.8",
    );
    let out = cache.path().join("out.txt");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/basic-resume:0.2.8")
        .arg("--format")
        .arg("text")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let body = std::fs::read_to_string(&out).expect("text output must be UTF-8");
    assert!(
        body.contains("Ada Lovelace"),
        "cached-preview text output must contain the rendered resume name; got: {body:?}"
    );
}

/// Cache hit, PDF dispatch path: the same fixture under `--format pdf`
/// produces a valid PDF stream. Pairs with the text-format test above
/// so both compile-target dispatch paths are exercised.
#[test]
fn render_resolves_preview_from_cache_pdf() {
    let cache = tempfile::TempDir::new().expect("tempdir cache");
    stage_cached_package(
        cache.path(),
        &cached_preview_fixture("basic-resume", "0.2.8"),
        "basic-resume",
        "0.2.8",
    );
    let out = cache.path().join("out.pdf");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/basic-resume:0.2.8")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let head = std::fs::read(&out)
        .expect("read PDF")
        .into_iter()
        .take(5)
        .collect::<Vec<_>>();
    assert_eq!(head, b"%PDF-", "output must start with the PDF magic bytes");
}

/// Cache miss: empty cache directory, render exits 2 with a stderr
/// message that contains both the spec and the install hint.
#[test]
fn render_preview_cache_miss_exits_two_with_install_hint() {
    let cache = tempfile::TempDir::new().expect("tempdir cache (intentionally empty)");
    let out = cache.path().join("out.pdf");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/missing-preview-package:9.9.9")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "@preview/missing-preview-package:9.9.9",
        ))
        .stderr(predicate::str::contains("ferrocv themes install"));

    assert!(
        !out.exists(),
        "no output file should be written on cache miss"
    );
}

/// CONSTITUTION §6.1 inline-import regression: even with the
/// surrounding theme resolved from the offline cache, an inline
/// `#import "@preview/..."` inside that theme's source still triggers
/// `FerrocvWorld::source` / `file` package rejection. The render
/// fails to compile (exit 2) with a diagnostic that mentions the
/// rejected package or "package" / "not found".
#[test]
fn preview_import_in_cached_theme_source_still_rejected() {
    let cache = tempfile::TempDir::new().expect("tempdir cache");
    stage_cached_package(
        cache.path(),
        &malicious_preview_fixture("imports-another-package", "1.0.0"),
        "imports-another-package",
        "1.0.0",
    );
    let out = cache.path().join("out.pdf");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/imports-another-package:1.0.0")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("preview")
                .or(predicate::str::contains("cetz"))
                .or(predicate::str::contains("package"))
                .or(predicate::str::contains("not found")),
        );

    assert!(
        !out.exists(),
        "no output file should be written on inline-import rejection"
    );
}

/// Sanity: a corrupt cache (manifest declares a different name than
/// the directory it lives in) is rejected with exit 2 and a clear
/// "remove the cache directory" hint. Catches a future bug where the
/// resolver would silently use the manifest's name instead of the
/// requested spec.
#[test]
fn render_preview_cache_corrupt_exits_two() {
    let cache = tempfile::TempDir::new().expect("tempdir cache");
    let pkg = cache
        .path()
        .join("packages")
        .join("preview")
        .join("asked-for")
        .join("1.0.0");
    std::fs::create_dir_all(pkg.join("src")).expect("mkdir staging");
    std::fs::write(
        pkg.join("typst.toml"),
        "[package]\nname = \"different-name\"\nversion = \"1.0.0\"\nentrypoint = \"src/lib.typ\"\n",
    )
    .expect("write manifest");
    std::fs::write(pkg.join("src/lib.typ"), "= Corrupt fixture\n").expect("write entrypoint");

    let out = cache.path().join("out.pdf");
    ferrocv()
        .env("FERROCV_CACHE_DIR", cache.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/asked-for:1.0.0")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("corrupt"));

    assert!(
        !out.exists(),
        "no output file should be written on cache corruption"
    );
}
