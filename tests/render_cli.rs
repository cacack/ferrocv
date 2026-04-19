//! Scenario-style black-box tests for the `ferrocv render` subcommand.
//!
//! These tests spawn the real built binary via `assert_cmd` and assert
//! on observable behavior only: exit code, stdout, stderr, and the
//! presence (and first few bytes) of the output file. They do not call
//! into the library API — `tests/render_theme.rs` covers that.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #1, every CLI-visible
//! behavior gets a scenario test, written before the implementation.
//! The exit-code contract under test:
//! - `0` — render succeeded; PDF written to `--output`
//! - `1` — JSON parsed but failed schema validation
//! - `2` — IO error, parse error, unknown theme, unknown format, or
//!   Typst render error
//!
//! The happy-path fixture is `render_full.json` rather than
//! `valid_full.json` because the vendored `typst-jsonresume-cv` theme
//! unconditionally reads fields (`meta.language`, `basics.location.region`,
//! `basics.phone`, `projects`) that the minimal `valid_full.json` fixture
//! does not carry. A richer fixture keeps theme-shape concerns out of
//! the generic validation fixture set.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

/// Absolute path to a fixture file by filename stem (no extension).
fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{name}.json"));
    path
}

/// Build a `Command` for the `ferrocv` binary under test.
fn ferrocv() -> Command {
    Command::cargo_bin("ferrocv").expect("binary `ferrocv` must be built")
}

/// Read the first `n` bytes of a file on disk.
fn read_prefix(path: &std::path::Path, n: usize) -> Vec<u8> {
    let bytes = std::fs::read(path).expect("output file must be readable");
    bytes.into_iter().take(n).collect()
}

#[test]
fn render_writes_pdf_to_output_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    assert_eq!(
        read_prefix(&out, 5),
        b"%PDF-",
        "output must start with the PDF magic bytes",
    );
    let size = std::fs::metadata(&out).expect("stat").len();
    assert!(size > 1024, "PDF should be > 1 KiB, was {size} bytes");
}

/// Regression guard for CONSTITUTION §1 ("JSON Resume unmodified"): any
/// resume that schema-validates must render, even if it omits optional
/// fields the adapter's upstream template happens to reference.
///
/// `render_sparse.json` carries only `basics.name`, a brief `summary`,
/// and one `work` entry — every other field is absent (no `meta`, no
/// `basics.location`, no `basics.email`/`phone`, no `projects`, no
/// `education`, no `skills`). JSON Resume v1.0.0 has zero required
/// fields, so this is a valid document; the adapter must cope.
#[test]
fn render_accepts_sparse_schema_valid_resume() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    ferrocv()
        .arg("render")
        .arg(fixture("render_sparse"))
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    assert_eq!(
        read_prefix(&out, 5),
        b"%PDF-",
        "output must start with the PDF magic bytes",
    );
    let size = std::fs::metadata(&out).expect("stat").len();
    assert!(size > 1024, "PDF should be > 1 KiB, was {size} bytes");
}

#[test]
fn render_rejects_unknown_theme_with_exit_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("nope")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("nope"))
        // A useful error lists what the user *could* have typed.
        .stderr(predicate::str::contains("typst-jsonresume-cv"));

    assert!(
        !out.exists(),
        "no output file should be written on unknown theme",
    );
}

#[test]
fn render_rejects_invalid_resume_with_exit_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    ferrocv()
        .arg("render")
        .arg(fixture("invalid_wrong_type_email"))
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("/basics/email"));

    assert!(
        !out.exists(),
        "no output file should be written when validation fails",
    );
}

#[test]
fn render_rejects_malformed_json_with_exit_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    ferrocv()
        .arg("render")
        .arg(fixture("malformed"))
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error:"));

    assert!(
        !out.exists(),
        "no output file should be written on parse failure",
    );
}

#[test]
fn render_reports_missing_input_file_with_exit_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");
    let missing = "/nonexistent/path/that/does/not/exist.json";

    ferrocv()
        .arg("render")
        .arg(missing)
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(missing));

    assert!(!out.exists());
}

#[test]
fn render_rejects_unknown_format_with_exit_two() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    // clap's ValueEnum mismatch exits 2 before we reach any of our code.
    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2);

    assert!(!out.exists());
}

#[test]
fn render_accepts_resume_on_stdin() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");
    let input =
        std::fs::read_to_string(fixture("render_full")).expect("fixture `render_full.json`");

    ferrocv()
        .arg("render")
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .write_stdin(input)
        .assert()
        .success();

    assert!(out.exists());
    assert_eq!(read_prefix(&out, 5), b"%PDF-");
}
