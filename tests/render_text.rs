//! Golden-file tests for the `text-minimal` native theme.
//!
//! Enforces CONSTITUTION testing doctrine:
//! - **§2** — every theme has a golden-file test. We compile the
//!   `text-minimal` native theme against committed fixtures via
//!   [`ferrocv::compile_text`] (the same path the `render --format text`
//!   CLI takes) and compare its normalized output against committed
//!   reference files. Where the PDF golden in `tests/render_theme.rs`
//!   defends against PDF-extraction regressions in an *adapter*, this
//!   golden defends against text-extraction regressions in our own
//!   *native* theme; both are first-class, per CONSTITUTION §3.
//! - **§4** — no mocking Typst. Runs the real embedded compiler.
//!
//! Mirrors the structure of `tests/render_theme.rs`; the small
//! `normalize()` helper is intentionally duplicated rather than
//! refactored into a shared module so the two test files stay
//! independently runnable and the no-shared-helper rule is preserved
//! per the prompt that introduced this file (CONSTITUTION §5: simple
//! now, iterate later — share when there's a third caller, not the
//! second).
//!
//! # Updating the goldens
//!
//! If a Typst patch bump or an intentional theme edit legitimately
//! shifts the rendered text, regenerate with:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --test render_text
//! ```
//!
//! Inspect the diffs under `tests/goldens/` before committing. A
//! golden update should always have an obvious cause; an unexplained
//! diff is a regression, not a golden bump.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// Environment variable that, when set to any non-empty value, rewrites
/// the relevant golden with the current run's normalized output
/// instead of asserting equality.
const UPDATE_ENV: &str = "UPDATE_GOLDEN";

#[test]
fn text_minimal_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "tests/fixtures/render_full.json",
        "tests/goldens/text-minimal.txt",
        "Ada Lovelace",
    );
}

#[test]
fn text_minimal_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "tests/fixtures/render_sparse.json",
        "tests/goldens/text-minimal-sparse.txt",
        "Grace Hopper",
    );
}

/// Shared golden-file workflow: compile `text-minimal` against the
/// named fixture, normalize the extracted text, and compare against
/// (or rewrite, under `UPDATE_GOLDEN`) the committed golden.
///
/// `required_name` is a substring the extracted text must contain
/// before we'll consider writing a golden from it — a cheap sanity
/// check that the extractor hasn't degenerated and we're not about to
/// freeze garbage.
fn run_golden(fixture: &str, golden: &str, required_name: &str) {
    let fixture_path = crate_path(fixture);
    let fixture_bytes = fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", fixture_path.display()));
    let data: Value = serde_json::from_slice(&fixture_bytes)
        .unwrap_or_else(|e| panic!("parse fixture {}: {e}", fixture_path.display()));

    let theme = ferrocv::find_theme("text-minimal").expect("theme must be registered in THEMES");

    let raw = ferrocv::compile_text(theme, &data).unwrap_or_else(|e| {
        panic!(
            "text-minimal must compile against {}: {e}",
            fixture_path.display()
        )
    });
    let normalized = normalize(&raw);

    // Sanity check runs BEFORE any golden write. If the extractor ever
    // returns empty or degenerate text, we refuse to regenerate the
    // golden from that garbage.
    assert!(
        normalized.contains(required_name),
        "extracted text must contain the fixture's name `{required_name}`; \
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
             UPDATE_GOLDEN=1 cargo test --test render_text",
            golden_path.display(),
        )
    });
    // Normalize the on-disk golden too so CRLF-delimited checkouts
    // (Windows + git autocrlf) match the always-LF output of
    // `normalize()`.
    let expected = normalize(&expected_raw);

    assert_eq!(
        normalized,
        expected,
        "text-minimal golden mismatch for {}. \
         Regenerate with `UPDATE_GOLDEN=1 cargo test --test render_text` \
         if the difference is expected (e.g., Typst version bump or an \
         intentional theme edit).",
        golden_path.display(),
    );
}

/// Normalize extracted text so the golden is stable against trivial
/// whitespace differences without hiding real regressions.
///
/// Kept deliberately narrow:
/// - trim trailing whitespace on each line
/// - collapse runs of blank lines to a single blank
/// - strip leading and trailing blank lines
/// - ensure exactly one trailing newline
///
/// Do NOT lowercase, Unicode-normalize, or collapse intra-line
/// whitespace — those transformations would hide regressions the
/// golden is meant to catch.
///
/// Duplicated verbatim from `tests/render_theme.rs::normalize`; see
/// the module-level docs for why we don't share it yet.
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
