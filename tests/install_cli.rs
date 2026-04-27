//! Scenario-style black-box tests for `ferrocv themes install`.
//!
//! Entire file is gated behind `#[cfg(feature = "install")]` — under
//! the default build (no features) the `Install` subcommand does not
//! exist, and this test file does not compile.
//!
//! The tests fall into two groups:
//!
//! 1. **Offline scenarios** (always run): spec parsing failures,
//!    cache-path idempotency, manifest mismatch rejection. These
//!    exercise the CLI end-to-end but never touch the network — the
//!    fixture tarball is assembled in-memory via `flate2 + tar` at
//!    test-setup time, served from a `std::net::TcpListener` bound
//!    to `127.0.0.1:0`, and the binary is pointed at that address
//!    via the internal `FERROCV_REGISTRY_URL` env var. No checked-in
//!    tarball bytes.
//! 2. **Live-network scenarios** (marked `#[ignore]`): exercise the
//!    real `packages.typst.org` endpoint. Opt in with
//!    `cargo test --features install -- --include-ignored`.

#![cfg(feature = "install")]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use assert_cmd::Command;
use flate2::Compression;
use flate2::write::GzEncoder;
use predicates::prelude::*;
use tar::{Builder, Header};

/// `std::env::set_var` is process-global; serialize tests that fiddle
/// with env vars so they do not race under the default parallel
/// runner. Using `OnceLock<Mutex<()>>` rather than a static `Mutex`
/// so we don't depend on `lazy_static` / `once_cell`.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Build a `Command` for the `ferrocv` binary with the `install`
/// feature enabled.
fn ferrocv() -> Command {
    Command::cargo_bin("ferrocv").expect("binary `ferrocv` must be built with --features install")
}

/// Construct an in-memory `.tar.gz` whose entries are
/// `(path, bytes)` pairs. Entries are written flat (no wrapper
/// directory) to match the Typst Universe convention.
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

/// Spawn a one-shot HTTP/1.1 server that serves a single body for
/// one `GET /<anything>` request with the chosen status, returning
/// the bind address.
///
/// The server lives on a dedicated thread, handles exactly one
/// connection, and exits. Good enough for per-test isolation; each
/// test spawns its own server on an ephemeral port.
fn spawn_fixture_server_with_status(body: Vec<u8>, status: u16, reason: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("addr").to_string();
    let reason = reason.to_owned();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = read_request_line(&mut stream);
            let headers = format!(
                "HTTP/1.1 {status} {reason}\r\n\
                 Content-Type: application/gzip\r\n\
                 Content-Length: {len}\r\n\
                 Connection: close\r\n\r\n",
                len = body.len(),
            );
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        }
    });
    addr
}

/// Drain the client's request until the blank line that ends its
/// headers. We don't parse anything — we just need the stream to be
/// drained so the client's write doesn't block before we reply.
fn read_request_line(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0u8; 1];
    let mut seen = 0u32;
    for _ in 0..8192 {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        match (seen, buf[0]) {
            (0, b'\r') => seen = 1,
            (1, b'\n') => seen = 2,
            (2, b'\r') => seen = 3,
            (3, b'\n') => return Ok(()),
            _ => seen = 0,
        }
    }
    Ok(())
}

/// Build a tarball for a valid `basic-resume`-style fixture that
/// declares the requested name/version in its `typst.toml` and ships
/// a one-line `src/lib.typ` entrypoint.
fn fixture_tarball(name: &str, version: &str) -> Vec<u8> {
    let toml_src = format!(
        "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"src/lib.typ\"\n",
    );
    build_tarball(&[
        ("typst.toml", toml_src.as_bytes()),
        ("src/lib.typ", b"#let version = \"0.0.0\"\n"),
    ])
}

/// Helper: install against a fixture server, returning the
/// `assert_cmd` `Assert` so the caller can chain assertions.
fn install_from_fixture(
    spec: &str,
    tarball: Vec<u8>,
    status: u16,
    reason: &str,
) -> (PathBuf, assert_cmd::assert::Assert) {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().expect("temp cache");
    let addr = spawn_fixture_server_with_status(tarball, status, reason);
    let registry = format!("http://{addr}");
    let assert = ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .env("FERROCV_REGISTRY_URL", &registry)
        .arg("themes")
        .arg("install")
        .arg(spec)
        .assert();
    (cache_dir.keep(), assert)
}

#[test]
fn install_rejects_malformed_spec() {
    // No @preview/ prefix at all.
    ferrocv()
        .arg("themes")
        .arg("install")
        .arg("basic-resume:0.2.8")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid package spec"))
        .stderr(predicate::str::contains("@preview/"));
}

#[test]
fn install_rejects_non_preview_namespace() {
    ferrocv()
        .arg("themes")
        .arg("install")
        .arg("@local/mine:1.0.0")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("only the @preview/ namespace"));
}

#[test]
fn install_rejects_missing_version() {
    ferrocv()
        .arg("themes")
        .arg("install")
        .arg("@preview/basic-resume")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("missing `:<version>`"));
}

#[test]
fn install_happy_path_writes_cache_and_prints_path() {
    let tarball = fixture_tarball("basic-resume", "0.2.8");
    let (cache_dir, assert) =
        install_from_fixture("@preview/basic-resume:0.2.8", tarball, 200, "OK");
    let expected_path = cache_dir
        .join("packages")
        .join("preview")
        .join("basic-resume")
        .join("0.2.8");
    let expected_path_str = expected_path.display().to_string();
    assert
        .success()
        .stdout(predicate::str::contains(expected_path_str.as_str()))
        .stderr(predicate::str::contains("installed"));
    assert!(
        expected_path.join("typst.toml").is_file(),
        "cached typst.toml should exist at {}",
        expected_path.display(),
    );
    assert!(
        expected_path.join("src").join("lib.typ").is_file(),
        "cached entrypoint should exist",
    );

    let _ = std::fs::remove_dir_all(&cache_dir);
}

#[test]
fn install_is_idempotent_on_second_run() {
    let tarball = fixture_tarball("basic-resume", "0.2.9");
    let (cache_dir, assert) =
        install_from_fixture("@preview/basic-resume:0.2.9", tarball, 200, "OK");
    assert.success();

    // Second run: no fixture server — a stray network call would
    // fail-closed because FERROCV_REGISTRY_URL points at a port the
    // first server has already dropped. Instead we expect the cache
    // hit to short-circuit before any fetch happens.
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    ferrocv()
        .env("FERROCV_CACHE_DIR", &cache_dir)
        .env("FERROCV_REGISTRY_URL", "http://127.0.0.1:1") // unreachable
        .arg("themes")
        .arg("install")
        .arg("@preview/basic-resume:0.2.9")
        .assert()
        .success()
        .stderr(predicate::str::contains("already cached"));

    let _ = std::fs::remove_dir_all(&cache_dir);
}

#[test]
fn install_rejects_manifest_mismatch() {
    // Tarball declares a different name than the spec asks for.
    let tarball = fixture_tarball("different-name", "0.2.8");
    let (cache_dir, assert) =
        install_from_fixture("@preview/basic-resume:0.2.8", tarball, 200, "OK");
    assert
        .failure()
        .code(2)
        .stderr(predicate::str::contains("manifest mismatch"));
    let final_dir = cache_dir
        .join("packages")
        .join("preview")
        .join("basic-resume")
        .join("0.2.8");
    assert!(
        !final_dir.exists(),
        "failed install must not publish cache entry",
    );

    let _ = std::fs::remove_dir_all(&cache_dir);
}

#[test]
fn install_surfaces_404_as_http_status_error() {
    let (cache_dir, assert) = install_from_fixture(
        "@preview/definitely-not-real:99.99.99",
        b"not found".to_vec(),
        404,
        "Not Found",
    );
    assert
        .failure()
        .code(2)
        .stderr(predicate::str::contains("HTTP 404"));

    let _ = std::fs::remove_dir_all(&cache_dir);
}

/// Live-network test: exercise the real `packages.typst.org` endpoint.
///
/// `#[ignore]`-by-default per the plan's test list — CI runs without
/// network access, so this only runs locally via
/// `cargo test --features install -- --include-ignored`.
#[test]
#[ignore]
fn install_fetches_live_package() {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().unwrap();
    ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .arg("themes")
        .arg("install")
        .arg("@preview/basic-resume:0.2.8")
        .assert()
        .success()
        .stderr(
            predicate::str::contains("installed").or(predicate::str::contains("already cached")),
        );
    assert!(
        cache_dir
            .path()
            .join("packages/preview/basic-resume/0.2.8/typst.toml")
            .is_file(),
    );
}
