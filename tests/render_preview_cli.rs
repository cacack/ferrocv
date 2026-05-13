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
//! - Uncached-transitive regression (issue #114 retighten): a cached
//!   package whose Typst source does `#import "@preview/cetz:0.2.0"`,
//!   where `cetz` is **not** in the cache, exits 2 with an install
//!   hint. CONSTITUTION §6.1 rejection narrowed to "package not in
//!   cache" instead of "any package import"; cached transitives now
//!   resolve via the World branch added in #114.
//! - Cached transitive resolution (issue #114): a primary whose source
//!   has a real `#import "@preview/helper:1.0.0"` resolves the helper
//!   from cache and the rendered output reflects the helper's exports.

#![cfg(feature = "install")]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use assert_cmd::Command;
use flate2::Compression;
use flate2::write::GzEncoder;
use predicates::prelude::*;
use tar::{Builder, Header};

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

/// Issue #114 retightened regression: post-#114, render-time
/// `@preview/...` imports resolve from the local cache when the
/// requested package is present. The §6.1 rejection survives, just
/// narrower: imports of packages **not in cache** still fail with a
/// structured "package not found" diagnostic plus an install hint that
/// names the missing spec.
///
/// This pins (a) that the World does not invent a network fetch path
/// on cache miss, and (b) that the rendered diagnostic carries the
/// actionable `ferrocv themes install @preview/<name>:<ver>` follow-up
/// the resolver-time path already surfaces for the *primary* spec.
#[test]
fn uncached_preview_import_in_cached_theme_source_still_rejected() {
    let cache = tempfile::TempDir::new().expect("tempdir cache");
    stage_cached_package(
        cache.path(),
        &malicious_preview_fixture("imports-another-package", "1.0.0"),
        "imports-another-package",
        "1.0.0",
    );
    let out = cache.path().join("out.pdf");

    // The fixture's `src/lib.typ` does `#import "@preview/cetz:0.2.0":
    // *"`. `cetz` is intentionally NOT staged in the cache, so the new
    // World branch falls through to the same `Package(NotFound)`
    // rejection. The diagnostic must (a) name the missing spec, (b)
    // carry the "package not found" signal, and (c) include the
    // install hint added by the render-diagnostic formatter.
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
        .stderr(predicate::str::contains("@preview/cetz:0.2.0"))
        .stderr(predicate::str::contains("package not found"))
        .stderr(predicate::str::contains(
            "ferrocv themes install @preview/cetz:0.2.0",
        ));

    assert!(
        !out.exists(),
        "no output file should be written on uncached-import rejection"
    );
}

/// Issue #114 positive scenario: a cached primary whose source does a
/// real `#import "@preview/transitive:1.0.0"` resolves the helper from
/// the same offline cache, the import succeeds, and the rendered text
/// reflects the helper's exported symbol.
///
/// Pre-stages both packages directly (no install round-trip) so the
/// test stays focused on the render-time World branch. The companion
/// `render_against_recursively_installed_primary_uses_real_import`
/// scenario below covers the install → render flow end-to-end.
#[test]
fn cached_preview_transitive_import_resolves_from_cache() {
    let cache = tempfile::TempDir::new().expect("tempdir cache");

    // Helper: exports a single string symbol the primary's source
    // splices into the rendered output, so the assertion can prove the
    // import actually executed (rather than the primary having silently
    // skipped it).
    let helper_root = cache.path().join("packages/preview/cv-helper/1.0.0");
    std::fs::create_dir_all(helper_root.join("src")).expect("mkdir helper");
    std::fs::write(
        helper_root.join("typst.toml"),
        "[package]\nname = \"cv-helper\"\nversion = \"1.0.0\"\nentrypoint = \"src/lib.typ\"\n",
    )
    .expect("write helper manifest");
    std::fs::write(
        helper_root.join("src/lib.typ"),
        "#let banner = \"HELPER-EXPORT-OK\"\n",
    )
    .expect("write helper lib");

    // Primary: imports the helper and emits the banner alongside the
    // resume name.
    let primary_root = cache.path().join("packages/preview/cv-with-helper/1.0.0");
    std::fs::create_dir_all(primary_root.join("src")).expect("mkdir primary");
    std::fs::write(
        primary_root.join("typst.toml"),
        "[package]\nname = \"cv-with-helper\"\nversion = \"1.0.0\"\nentrypoint = \"src/lib.typ\"\n",
    )
    .expect("write primary manifest");
    std::fs::write(
        primary_root.join("src/lib.typ"),
        "#import \"@preview/cv-helper:1.0.0\": banner\n\
         #let resume = json(\"/resume.json\")\n\
         = #resume.basics.name\n\
         #banner\n",
    )
    .expect("write primary lib");

    let out = cache.path().join("out.txt");
    ferrocv()
        .env("FERROCV_CACHE_DIR", cache.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/cv-with-helper:1.0.0")
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
        body.contains("HELPER-EXPORT-OK"),
        "rendered text must contain the helper's exported banner; got: {body:?}"
    );
    assert!(
        body.contains("Ada Lovelace"),
        "rendered text must contain the resume name; got: {body:?}"
    );
}

// ---------------------------------------------------------------------
// Render-after-recursive-install scenario
//
// CONSTITUTION §5: third caller is the trigger to extract a shared
// helper. With only this file and `tests/install_cli.rs` needing the
// multi-route fixture server, we deliberately duplicate the helper
// rather than introduce a `tests/common/` module just for two callers.
// ---------------------------------------------------------------------

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn build_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = Builder::new(&mut gz);
        for (path, bytes) in entries {
            let mut header = Header::new_gnu();
            header.set_path(path).expect("valid tar path");
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append(&header, *bytes).expect("append entry");
        }
        tar.finish().expect("finalize tar");
    }
    gz.finish().expect("finalize gzip")
}

fn read_request_path(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
    loop {
        let mut next = String::new();
        let read = reader.read_line(&mut next)?;
        if read == 0 || next == "\r\n" || next == "\n" {
            break;
        }
    }
    Ok(path)
}

fn spawn_multi_route_fixture_server(routes: Vec<(String, u16, Vec<u8>)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("addr").to_string();
    let n = routes.len();
    std::thread::spawn(move || {
        for _ in 0..n {
            if let Ok((mut stream, _)) = listener.accept() {
                let path = read_request_path(&mut stream).unwrap_or_default();
                let key = path.trim_start_matches('/').to_owned();
                match routes.iter().find(|(p, _, _)| *p == key) {
                    Some((_, status, body)) => {
                        let reason = if *status == 200 { "OK" } else { "Error" };
                        let headers = format!(
                            "HTTP/1.1 {status} {reason}\r\n\
                             Content-Type: application/gzip\r\n\
                             Content-Length: {len}\r\n\
                             Connection: close\r\n\r\n",
                            len = body.len(),
                        );
                        let _ = stream.write_all(headers.as_bytes());
                        let _ = stream.write_all(body);
                    }
                    None => {
                        let _ = stream.write_all(
                            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                    }
                }
                let _ = stream.flush();
            }
        }
    });
    addr
}

/// `lib.typ` body for fixture `a`. Issue #114 unlocked render-time
/// resolution of `@preview/...` imports from the offline cache, so
/// this fixture now uses a **real** `#import` rather than a
/// string-literal decoy. The install-time `imports.rs` scanner still
/// picks it up (it parses `#import "@preview/..."` directives, not
/// just string literals); the render-time `FerrocvWorld` branch added
/// in #114 resolves the import from cache once `b` has been hydrated
/// by `themes install`. The exported `b_banner` symbol gives the
/// scenario test something concrete to assert against.
const A_LIB_TYP: &str = r##"// auto-generated test fixture for ferrocv recursive-install scenario
#import "@preview/b:2.0.0": b_banner
#let resume = json("/resume.json")
= #resume.basics.name
#b_banner
"##;

/// `lib.typ` body for fixture `b`: leaf, no further imports. Exports a
/// banner string the primary splices into the rendered output so the
/// install → render assertion can prove the transitive import actually
/// executed (rather than the primary having silently elided it).
const B_LIB_TYP: &str = r##"// auto-generated test fixture for ferrocv recursive-install scenario
#let b_banner = "TRANSITIVE-INSTALL-RENDER-OK"
"##;

fn fixture_tarball_with_lib(name: &str, version: &str, lib_body: &str) -> Vec<u8> {
    let toml_src = format!(
        "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"src/lib.typ\"\n",
    );
    build_tarball(&[
        ("typst.toml", toml_src.as_bytes()),
        ("src/lib.typ", lib_body.as_bytes()),
    ])
}

/// Offline `render` succeeds against a recursively installed primary
/// that imports a transitive helper at render time. Issue #114 broadened
/// the user-facing benefit of recursive install: the install step
/// hydrates primary + transitives in one network-permitted invocation,
/// and the render-time World resolves the real `@preview/...` import
/// from cache — no re-fetch, no separate user step per dep.
///
/// The fixture `a` does a **real** `#import "@preview/b:2.0.0": b_banner`
/// (post-#114, no longer a string-literal decoy) and emits `b_banner`
/// inline. The assertion proves both the cache hydration **and** the
/// render-time import resolution: if either failed, the banner would be
/// absent from the rendered text.
///
/// Two phases:
///
/// 1. `ferrocv themes install @preview/a:1.0.0` against a multi-route
///    fixture server. Exits 0; both `a` and `b` cached on disk.
/// 2. `ferrocv render --theme @preview/a:1.0.0` with NO registry
///    pointer set. Must succeed using only the local cache, with the
///    rendered text containing the helper's exported banner.
#[test]
fn render_against_recursively_installed_primary_uses_real_import() {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().expect("temp cache");

    // Phase 1: recursive install.
    let a_tarball = fixture_tarball_with_lib("a", "1.0.0", A_LIB_TYP);
    let b_tarball = fixture_tarball_with_lib("b", "2.0.0", B_LIB_TYP);
    let routes = vec![
        ("a-1.0.0.tar.gz".to_owned(), 200, a_tarball),
        ("b-2.0.0.tar.gz".to_owned(), 200, b_tarball),
    ];
    let addr = spawn_multi_route_fixture_server(routes);
    let registry = format!("http://{addr}");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .env("FERROCV_REGISTRY_URL", &registry)
        .arg("themes")
        .arg("install")
        .arg("@preview/a:1.0.0")
        .assert()
        .success();

    let cached_a = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("a")
        .join("1.0.0");
    let cached_b = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("b")
        .join("2.0.0");
    assert!(cached_a.join("typst.toml").is_file(), "a must be cached");
    assert!(cached_b.join("typst.toml").is_file(), "b must be cached");

    // Phase 2: offline render. NO `FERROCV_REGISTRY_URL` set; the
    // fixture server thread has already exited so any network attempt
    // from `render` would fail fast.
    let out = cache_dir.path().join("out.txt");
    ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .env_remove("FERROCV_REGISTRY_URL")
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("@preview/a:1.0.0")
        .arg("--format")
        .arg("text")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(
        out.exists(),
        "offline render must produce output at {}",
        out.display(),
    );
    let body = std::fs::read_to_string(&out).expect("text output must be UTF-8");
    assert!(
        body.contains("TRANSITIVE-INSTALL-RENDER-OK"),
        "render-time @preview/b:2.0.0 import must resolve from cache and surface b_banner in output; got: {body:?}",
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
