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
    // `xml` stands in for "any format we don't ship"; `html` was the
    // pre-#44 canary, now a valid format, so `xml` is the replacement.
    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("typst-jsonresume-cv")
        .arg("--format")
        .arg("xml")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2);

    assert!(!out.exists());
}

// --- Text format scenarios ---------------------------------------------
//
// Mirror the PDF scenarios above for `--format text`. The `text-minimal`
// native theme is exercised end-to-end through the CLI (file existence,
// recognizable content, exit-code contract). Byte-level structure of the
// extracted text is asserted by the golden test in `tests/render_text.rs`,
// not here.

#[test]
fn render_writes_text_to_output_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.txt");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("text-minimal")
        .arg("--format")
        .arg("text")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let body = std::fs::read_to_string(&out).expect("text output must be UTF-8");
    assert!(!body.is_empty(), "text output must not be empty");
    assert!(
        body.contains("Ada Lovelace"),
        "text output must contain the rendered name; got: {body:?}"
    );
}

#[test]
fn render_text_uses_text_minimal_when_theme_omitted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.txt");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
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
        "text output must contain the rendered name; got: {body:?}"
    );
}

#[test]
fn render_text_default_output_path_is_dist_resume_txt() {
    // No `--output`, no `--theme`. `current_dir` is set to a tempdir so
    // the default `dist/resume.txt` lands under the temp tree rather
    // than polluting the workspace.
    let tmp = tempfile::tempdir().expect("tempdir");

    ferrocv()
        .current_dir(tmp.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--format")
        .arg("text")
        .assert()
        .success();

    let expected = tmp.path().join("dist/resume.txt");
    assert!(
        expected.exists(),
        "default text output must land at <cwd>/dist/resume.txt; expected {}",
        expected.display()
    );
}

#[test]
fn render_text_rejects_invalid_resume_with_exit_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.txt");

    ferrocv()
        .arg("render")
        .arg(fixture("invalid_wrong_type_email"))
        .arg("--format")
        .arg("text")
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
fn render_pdf_requires_theme_flag() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.pdf");

    // No `--theme`; explicit `--format pdf` (also the default).
    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--format")
        .arg("pdf")
        .arg("--output")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--theme is required"));

    assert!(
        !out.exists(),
        "no output file should be written when --theme is missing",
    );
}

#[test]
fn render_text_accepts_resume_on_stdin() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.txt");
    let input =
        std::fs::read_to_string(fixture("render_full")).expect("fixture `render_full.json`");

    ferrocv()
        .arg("render")
        .arg("--format")
        .arg("text")
        .arg("--output")
        .arg(&out)
        .write_stdin(input)
        .assert()
        .success();

    assert!(out.exists());
    let body = std::fs::read_to_string(&out).expect("text output must be UTF-8");
    assert!(
        body.contains("Ada Lovelace"),
        "text output must contain the rendered name; got: {body:?}"
    );
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

// --- HTML format scenarios ---------------------------------------------
//
// Mirror the PDF and text scenarios above for `--format html`. HTML is
// Typst's upstream-experimental export, so assertions stay loose —
// exact byte shape is guaranteed to churn across Typst minors. The
// full-document well-formedness check (DOCTYPE presence, no external
// references, etc.) lives in `tests/render_html.rs`.

#[test]
fn render_writes_html_to_output_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.html");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("text-minimal")
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let body = std::fs::read_to_string(&out).expect("HTML output must be UTF-8");
    assert!(
        body.starts_with("<!DOCTYPE html>"),
        "HTML output must begin with the DOCTYPE declaration; got first 80 bytes: {:?}",
        body.chars().take(80).collect::<String>(),
    );
    assert!(
        body.contains("Ada Lovelace"),
        "HTML output must contain the rendered name; got {} bytes",
        body.len(),
    );
}

#[test]
fn render_html_uses_text_minimal_when_theme_omitted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.html");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let body = std::fs::read_to_string(&out).expect("HTML output must be UTF-8");
    assert!(
        body.contains("Ada Lovelace"),
        "HTML output must contain the rendered name"
    );
}

#[test]
fn render_html_default_output_path_is_dist_resume_html() {
    // No `--output`, no `--theme`. `current_dir` is set to a tempdir so
    // the default `dist/resume.html` lands under the temp tree rather
    // than polluting the workspace.
    let tmp = tempfile::tempdir().expect("tempdir");

    ferrocv()
        .current_dir(tmp.path())
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--format")
        .arg("html")
        .assert()
        .success();

    let expected = tmp.path().join("dist/resume.html");
    assert!(
        expected.exists(),
        "default HTML output must land at <cwd>/dist/resume.html; expected {}",
        expected.display()
    );
}

#[test]
fn render_html_rejects_invalid_resume_with_exit_one() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.html");

    ferrocv()
        .arg("render")
        .arg(fixture("invalid_wrong_type_email"))
        .arg("--format")
        .arg("html")
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
fn render_html_accepts_resume_on_stdin() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.html");
    let input =
        std::fs::read_to_string(fixture("render_full")).expect("fixture `render_full.json`");

    ferrocv()
        .arg("render")
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out)
        .write_stdin(input)
        .assert()
        .success();

    assert!(out.exists());
    let body = std::fs::read_to_string(&out).expect("HTML output must be UTF-8");
    assert!(
        body.contains("Ada Lovelace"),
        "HTML output must contain the rendered name"
    );
}

/// Adapter HTML happy-path: `fantastic-cv` was fixed for HTML
/// compatibility in #59 (empty-URL guard); verify the fix extends to
/// the CLI render path. Assertions are minimal — we only check the
/// compile succeeds and produces an HTML file. Shape of that output
/// is upstream-experimental and not something we want to pin.
#[test]
fn render_html_fantastic_cv_happy_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.html");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("fantastic-cv")
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let size = std::fs::metadata(&out).expect("stat").len();
    assert!(
        size > 0,
        "HTML output should be non-empty, was {size} bytes"
    );
}

/// Adapter HTML happy-path: `modern-cv` (added in #58) compiles to
/// HTML cleanly. Same minimal-assertion posture as the fantastic-cv
/// case — we only assert compile success and non-empty output.
#[test]
fn render_html_modern_cv_happy_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.html");

    ferrocv()
        .arg("render")
        .arg(fixture("render_full"))
        .arg("--theme")
        .arg("modern-cv")
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert!(out.exists(), "output file must exist at {}", out.display());
    let size = std::fs::metadata(&out).expect("stat").len();
    assert!(
        size > 0,
        "HTML output should be non-empty, was {size} bytes"
    );
}
