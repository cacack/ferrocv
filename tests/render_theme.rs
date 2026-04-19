//! Golden-file tests for the `typst-jsonresume-cv` theme adapter.
//!
//! Enforces CONSTITUTION testing doctrine:
//! - **§2** — every theme has a golden-file test. We compile the
//!   adapter against a committed fixture, extract the rendered text
//!   with `pdf-extract`, and compare against a committed reference.
//!   PDF bytes are fragile across Typst patch versions; a normalized
//!   text extraction is substantially more stable and still catches
//!   real regressions (missing sections, re-ordered fields, lost
//!   fields, etc.).
//! - **§4** — no mocking Typst. These tests drive the real embedded
//!   compiler via [`ferrocv::compile_theme`] against the real vendored
//!   theme sources. No stubs, no feature-gated "lite" mode.
//!
//! Also indirectly exercises **§6.1** (no network at render time): the
//! adapter imports nothing under `@preview/...`, and `compile_theme`
//! goes through the same [`FerrocvWorld`] that rejects package
//! coordinates unconditionally. If a future vendored-theme update
//! introduced a network-requiring import, these tests would fail
//! loudly (the compile would error) rather than silently succeeding.
//!
//! # Fixture coverage
//!
//! Two fixtures are exercised, each pinned to its own golden so that a
//! regression in either "shape" shows up as a committed diff:
//!
//! - `render_full.json` (Ada Lovelace) — every field the theme reads
//!   is populated, including all optional contact fields, all section
//!   arrays, and `meta.language`. Pins the "full-field" output shape.
//! - `render_sparse.json` (Grace Hopper) — a schema-valid but
//!   deliberately-sparse document: no `meta`, no `basics.location`,
//!   no `basics.email`/`phone`, no `projects`/`education`/`skills`.
//!   Pins the optional-field shim in
//!   `assets/themes/typst-jsonresume-cv/resume.typ`
//!   (see that file's `VENDORING.md` patch #3). A silent regression
//!   in how missing fields degrade becomes a committed diff here.
//!
//! Fixture contents use public-domain pioneers of computing to keep
//! test data PII-free and recognizable at a glance. These fixtures are
//! *test-shaped* — narrow, deliberate coverage of specific adapter
//! code paths. A realistic, demo-quality resume (the kind a user
//! would actually author) belongs in the forkable starter-template
//! repo tracked by issue #43, not here.
//!
//! # Updating the goldens
//!
//! If a Typst patch bump or an intentional theme edit legitimately
//! shifts the rendered text, regenerate with:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --test render_theme
//! ```
//!
//! Inspect the diffs under `tests/goldens/` before committing. A
//! golden update should always have an obvious cause; an unexplained
//! diff is a regression, not a golden bump.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// PDF magic bytes; every valid PDF stream must start with `%PDF-`.
const PDF_MAGIC: &[u8] = b"%PDF-";

/// Environment variable that, when set to any non-empty value, rewrites
/// the relevant golden with the current run's normalized output
/// instead of asserting equality.
const UPDATE_ENV: &str = "UPDATE_GOLDEN";

#[test]
fn typst_jsonresume_cv_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "tests/fixtures/render_full.json",
        "tests/goldens/typst-jsonresume-cv.txt",
        "Ada Lovelace",
    );
}

#[test]
fn typst_jsonresume_cv_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "tests/fixtures/render_sparse.json",
        "tests/goldens/typst-jsonresume-cv-sparse.txt",
        "Grace Hopper",
    );
}

/// Shared golden-file workflow: compile the adapter against the named
/// fixture, extract and normalize the PDF text, and compare against
/// (or rewrite, under `UPDATE_GOLDEN`) the committed golden.
///
/// `required_name` is a substring the extracted text must contain
/// before we'll consider writing a golden from it — a cheap sanity
/// check that `pdf-extract` hasn't degenerated and we're not about to
/// freeze garbage.
fn run_golden(fixture: &str, golden: &str, required_name: &str) {
    let fixture_path = crate_path(fixture);
    let fixture_bytes = fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", fixture_path.display()));
    let data: Value = serde_json::from_slice(&fixture_bytes)
        .unwrap_or_else(|e| panic!("parse fixture {}: {e}", fixture_path.display()));

    let theme =
        ferrocv::find_theme("typst-jsonresume-cv").expect("theme must be registered in THEMES");

    let bytes = ferrocv::compile_theme(theme, &data).unwrap_or_else(|e| {
        panic!(
            "typst-jsonresume-cv must compile against {}: {e}",
            fixture_path.display()
        )
    });

    assert!(
        bytes.starts_with(PDF_MAGIC),
        "compiled output must begin with PDF magic; got first 16 bytes: {:?}",
        &bytes[..bytes.len().min(16)],
    );
    assert!(
        bytes.len() > 1024,
        "PDF output suspiciously small ({} bytes) — expected a real multi-page resume",
        bytes.len(),
    );

    let raw = pdf_extract::extract_text_from_mem(&bytes)
        .expect("pdf-extract must parse compiled PDF bytes");
    let normalized = normalize(&raw);

    // Sanity check runs BEFORE any golden write. If the extractor ever
    // returns empty or degenerate text, we refuse to regenerate the
    // golden from that garbage.
    assert!(
        normalized.contains(required_name),
        "extracted PDF text must contain the fixture's name `{required_name}`; \
         got {} bytes of normalized text starting with:\n{}",
        normalized.len(),
        normalized.chars().take(200).collect::<String>(),
    );

    let golden_path = crate_path(golden);

    if std::env::var_os(UPDATE_ENV).is_some_and(|v| !v.is_empty()) {
        if let Some(parent) = golden_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("create {}: {e}", parent.display()));
        }
        fs::write(&golden_path, &normalized)
            .unwrap_or_else(|e| panic!("write golden {}: {e}", golden_path.display()));
        eprintln!(
            "{UPDATE_ENV} set: wrote {} bytes to {}. \
             Re-run without {UPDATE_ENV} to verify.",
            normalized.len(),
            golden_path.display(),
        );
        return;
    }

    let expected_raw = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!(
            "read golden {}: {e}\nTo create it, run: \
             UPDATE_GOLDEN=1 cargo test --test render_theme",
            golden_path.display(),
        )
    });
    // Normalize the on-disk golden too so CRLF-delimited checkouts
    // (Windows + git autocrlf) match the always-LF output of
    // `normalize()`. `.gitattributes` pins the file to LF for new
    // checkouts, but pre-existing clones may still have CRLF.
    let expected = normalize(&expected_raw);

    assert_eq!(
        normalized,
        expected,
        "theme golden mismatch for {}. \
         Regenerate with `UPDATE_GOLDEN=1 cargo test --test render_theme` \
         if the difference is expected (e.g., Typst version bump).",
        golden_path.display(),
    );
}

/// Normalize extracted PDF text so the golden is stable against
/// trivial whitespace differences without hiding real regressions.
///
/// Kept deliberately narrow:
/// - trim trailing whitespace on each line
/// - collapse runs of blank lines to a single blank
/// - strip leading and trailing blank lines
/// - ensure exactly one trailing newline
///
/// Do NOT lowercase, Unicode-normalize, or collapse intra-line
/// whitespace — those transformations would hide rendering
/// regressions the golden is meant to catch.
fn normalize(raw: &str) -> String {
    let mut lines: Vec<String> = raw.lines().map(|l| l.trim_end().to_string()).collect();
    let mut collapsed: Vec<String> = Vec::with_capacity(lines.len());
    let mut prev_blank = false;
    for line in lines.drain(..) {
        let is_blank = line.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        collapsed.push(line);
        prev_blank = is_blank;
    }
    while collapsed.first().is_some_and(|s| s.is_empty()) {
        collapsed.remove(0);
    }
    while collapsed.last().is_some_and(|s| s.is_empty()) {
        collapsed.pop();
    }
    let mut out = collapsed.join("\n");
    out.push('\n');
    out
}

/// Resolve a path relative to the crate root (where `Cargo.toml`
/// lives). Cargo sets `CARGO_MANIFEST_DIR` for integration tests, so
/// this works regardless of the shell's `cwd` at test time.
fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}
