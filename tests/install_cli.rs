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

use std::io::{BufRead, BufReader, Read, Write};
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

/// Spawn an HTTP/1.1 server that serves a small route table over up
/// to `routes.len()` connections. Each request's URL path (the
/// `/<file>` portion of `GET /<file> HTTP/1.1`) is matched against the
/// route table's first column; on a hit the configured status+body is
/// served, otherwise a 404 with empty body is served. The server
/// thread terminates after handling the expected number of
/// connections.
///
/// Route key is the URL-path tail (`<name>-<version>.tar.gz`) — the
/// `fetch.rs::tarball_url` builder appends that tail to the registry
/// root. So a route registered as `("basic-resume-0.2.8.tar.gz", 200,
/// body)` matches the request `GET /basic-resume-0.2.8.tar.gz`.
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

/// Read the HTTP request line and drain headers. Returns the URL path
/// (e.g. `/basic-resume-0.2.8.tar.gz` from `GET /basic-resume-0.2.8.tar.gz HTTP/1.1`).
fn read_request_path(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    // `GET /<path> HTTP/1.1\r\n`
    let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
    // Drain remaining headers up to the blank line so the client's
    // write doesn't block before we reply.
    loop {
        let mut next = String::new();
        let read = reader.read_line(&mut next)?;
        if read == 0 || next == "\r\n" || next == "\n" {
            break;
        }
    }
    Ok(path)
}

/// Build a fixture tarball whose `src/lib.typ` "declares" each
/// `(dep_name, dep_version)` as a string literal. The `imports.rs`
/// scanner picks these strings up as transitive `@preview/...` specs
/// regardless of surrounding Typst grammar; using string literals
/// rather than real `#import` statements keeps render-time tests
/// independent of the `FerrocvWorld` `@preview/...` rejection (which
/// would refuse to compile a real inline import).
fn fixture_tarball_with_imports(name: &str, version: &str, imports: &[(&str, &str)]) -> Vec<u8> {
    let toml_src = format!(
        "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"src/lib.typ\"\n",
    );
    let mut lib = String::from("// auto-generated test fixture\n");
    for (dep_name, dep_version) in imports {
        lib.push_str(&format!(
            "#let _annotation_{dep_name} = \"@preview/{dep_name}:{dep_version}\"\n",
        ));
    }
    lib.push_str("// end\n");
    build_tarball(&[
        ("typst.toml", toml_src.as_bytes()),
        ("src/lib.typ", lib.as_bytes()),
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

/// Recursive install: a primary package whose source declares a
/// single transitive `@preview/...` reference. The driver must fetch
/// both packages and the CLI must report the transitive in its stderr
/// summary (tagged `[installed]`).
#[test]
fn install_recursively_fetches_one_transitive_dep() {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().expect("temp cache");
    let a_tarball = fixture_tarball_with_imports("a", "1.0.0", &[("b", "2.0.0")]);
    let b_tarball = fixture_tarball_with_imports("b", "2.0.0", &[]);
    let routes = vec![
        ("a-1.0.0.tar.gz".to_owned(), 200, a_tarball),
        ("b-2.0.0.tar.gz".to_owned(), 200, b_tarball),
    ];
    let addr = spawn_multi_route_fixture_server(routes);
    let registry = format!("http://{addr}");

    let expected_a = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("a")
        .join("1.0.0");
    let expected_b = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("b")
        .join("2.0.0");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .env("FERROCV_REGISTRY_URL", &registry)
        .arg("themes")
        .arg("install")
        .arg("@preview/a:1.0.0")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            expected_a.display().to_string().as_str(),
        ))
        .stderr(predicate::str::contains("installed @preview/a:1.0.0"))
        .stderr(predicate::str::contains(
            "also installed 1 transitive dep(s):",
        ))
        .stderr(predicate::str::contains("@preview/b:2.0.0"))
        .stderr(predicate::str::contains("[installed]"));

    assert!(
        expected_a.join("typst.toml").is_file(),
        "primary's typst.toml must be cached at {}",
        expected_a.display(),
    );
    assert!(
        expected_b.join("typst.toml").is_file(),
        "transitive dep's typst.toml must be cached at {}",
        expected_b.display(),
    );
}

/// Recursive install with a cycle in the declared `@preview/...`
/// imports must terminate cleanly via the visited set: each package
/// is fetched once, the cycle is silent in the summary, and both
/// packages end up cached on disk.
#[test]
fn install_recursive_handles_cycle() {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().expect("temp cache");
    let a_tarball = fixture_tarball_with_imports("a", "1.0.0", &[("b", "2.0.0")]);
    let b_tarball = fixture_tarball_with_imports("b", "2.0.0", &[("a", "1.0.0")]);
    // Two routes — each served at most once. If the visited set is
    // broken and the driver re-requests one, the server will refuse
    // (already exited) and the test will fail loudly.
    let routes = vec![
        ("a-1.0.0.tar.gz".to_owned(), 200, a_tarball),
        ("b-2.0.0.tar.gz".to_owned(), 200, b_tarball),
    ];
    let addr = spawn_multi_route_fixture_server(routes);
    let registry = format!("http://{addr}");

    let expected_a = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("a")
        .join("1.0.0");
    let expected_b = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("b")
        .join("2.0.0");

    let output = ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .env("FERROCV_REGISTRY_URL", &registry)
        .arg("themes")
        .arg("install")
        .arg("@preview/a:1.0.0")
        .assert()
        .success();

    assert!(
        expected_a.join("typst.toml").is_file(),
        "primary cached at {}",
        expected_a.display(),
    );
    assert!(
        expected_b.join("typst.toml").is_file(),
        "transitive cached at {}",
        expected_b.display(),
    );

    // Cycle must not cause `b` to appear more than once in the
    // transitive summary.
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).into_owned();
    let occurrences = stderr.matches("@preview/b:2.0.0").count();
    assert_eq!(
        occurrences, 1,
        "cycle must yield exactly one mention of @preview/b:2.0.0 in stderr; got {occurrences} in:\n{stderr}",
    );
}

/// Recursive install hard-fails when a transitive 404s. Exit code is 2,
/// stderr names the parent + child + inner cause, and the primary's
/// cache entry is left in place per the prompt-001 contract.
#[test]
fn install_recursive_hard_fails_when_transitive_404s() {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().expect("temp cache");
    let a_tarball = fixture_tarball_with_imports("a", "1.0.0", &[("missing", "9.9.9")]);
    let routes = vec![
        ("a-1.0.0.tar.gz".to_owned(), 200, a_tarball),
        (
            "missing-9.9.9.tar.gz".to_owned(),
            404,
            b"not found".to_vec(),
        ),
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
        .code(2)
        .stderr(predicate::str::contains(
            "failed to install transitive dep @preview/missing:9.9.9 required by @preview/a:1.0.0",
        ))
        .stderr(predicate::str::contains("404"))
        .stderr(predicate::str::contains(
            "primary @preview/a:1.0.0 remains cached",
        ));

    let primary_path = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("a")
        .join("1.0.0");
    // The pipeline's `ensure_parent_exists` mkdirs the package's
    // parent directory (`packages/preview/missing/`) before the fetch
    // is attempted — so the parent dir may exist as an empty
    // breadcrumb. The assertion under test is that the *versioned*
    // package directory does NOT exist (the only thing that would
    // satisfy a future cache-hit short-circuit).
    let missing_versioned_path = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("missing")
        .join("9.9.9");
    assert!(
        primary_path.join("typst.toml").is_file(),
        "primary's cache entry must remain after a transitive failure (at {})",
        primary_path.display(),
    );
    assert!(
        !missing_versioned_path.exists(),
        "failed transitive must not leave a versioned cache entry behind (at {})",
        missing_versioned_path.display(),
    );
}

/// Backward-compatibility: when a recursively-installed package has
/// zero transitive deps, stderr matches the existing single-package
/// summary text exactly. The "transitive dep(s)" summary line is
/// suppressed entirely so older scripts that expected the simpler
/// stderr shape keep working.
#[test]
fn install_no_transitives_keeps_existing_summary_text() {
    let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let cache_dir = tempfile::TempDir::new().expect("temp cache");
    let a_tarball = fixture_tarball_with_imports("a", "1.0.0", &[]);
    let routes = vec![("a-1.0.0.tar.gz".to_owned(), 200, a_tarball)];
    let addr = spawn_multi_route_fixture_server(routes);
    let registry = format!("http://{addr}");

    let expected_a = cache_dir
        .path()
        .join("packages")
        .join("preview")
        .join("a")
        .join("1.0.0");

    ferrocv()
        .env("FERROCV_CACHE_DIR", cache_dir.path())
        .env("FERROCV_REGISTRY_URL", &registry)
        .arg("themes")
        .arg("install")
        .arg("@preview/a:1.0.0")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            expected_a.display().to_string().as_str(),
        ))
        .stderr(predicate::str::contains("installed @preview/a:1.0.0"))
        .stderr(predicate::str::contains("transitive dep(s)").not());
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
