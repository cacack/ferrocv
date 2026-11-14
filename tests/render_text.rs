//! Golden-file tests for `--format text` output.
//!
//! Enforces CONSTITUTION testing doctrine:
//! - **§2** — every theme has a golden-file test. We compile the
//!   `text-minimal` and `classic` native themes against committed
//!   fixtures via
//!   [`ferrocv::compile_text`] (the same path the `render --format text`
//!   CLI takes) and compare its normalized output against committed
//!   reference files. Where the PDF golden in `tests/render_theme.rs`
//!   defends against PDF-extraction regressions in an *adapter*, this
//!   golden defends against text-extraction regressions in our own
//!   *native* themes; both are first-class, per CONSTITUTION §3.
//!   `classic` is PDF-first, so its `tests/render_theme.rs` golden
//!   goes through `pdf-extract`; the `classic-text*.txt` goldens here
//!   pin what `render --format text --theme classic` actually emits.
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
        "text-minimal",
        "tests/fixtures/render_full.json",
        "tests/goldens/text-minimal.txt",
        "Ada Lovelace",
    );
}

#[test]
fn text_minimal_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "text-minimal",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/text-minimal-sparse.txt",
        "Grace Hopper",
    );
}

#[test]
fn classic_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "classic",
        "tests/fixtures/render_full.json",
        "tests/goldens/classic-text.txt",
        "Ada Lovelace",
    );
}

#[test]
fn classic_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "classic",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/classic-text-sparse.txt",
        "Grace Hopper",
    );
}

/// Two work entries, the first with a summary long enough to wrap and
/// two highlights — the shapes issue #292's line grouping must keep
/// apart from section and entry breaks.
fn two_entry_resume() -> Value {
    serde_json::json!({
        "basics": { "name": "Ada Lovelace" },
        "work": [
            {
                "name": "Analytical Engines Ltd",
                "position": "Programmer",
                "startDate": "1843-01-01",
                "endDate": "1852-11-27",
                "summary": "Translated and annotated Menabrea's paper on the Analytical \
                            Engine, adding notes three times the length of the original \
                            and the first published algorithm intended for a machine.",
                "highlights": ["First highlight", "Second highlight"]
            },
            { "name": "Difference Engine Society", "position": "Correspondent" }
        ]
    })
}

fn render_text(theme_name: &str, data: &Value) -> String {
    let theme = ferrocv::find_theme(theme_name).expect("theme must be registered");
    ferrocv::compile_text(theme, data).expect("theme must compile")
}

/// Given an entry whose summary wraps and which has two highlights,
/// when `text-minimal` renders it, then the wrapped summary and the
/// highlights are contiguous lines, while blank lines still separate
/// the section heading and the two entries.
#[test]
fn text_minimal_keeps_paragraphs_and_bullets_contiguous() {
    let out = render_text("text-minimal", &two_entry_resume());
    let blocks: Vec<&str> = out.trim_end().split("\n\n").collect();
    let first_entry = blocks
        .iter()
        .find(|b| b.starts_with("Programmer - Analytical Engines Ltd"))
        .unwrap_or_else(|| panic!("first entry must be one block; got:\n{out}"));
    let lines: Vec<&str> = first_entry.lines().collect();
    assert!(
        lines.len() >= 6,
        "header, dates, wrapped summary (2+ lines) and two bullets must \
         share one block; got:\n{out}"
    );
    assert_eq!(lines[lines.len() - 2], "- First highlight", "got:\n{out}");
    assert_eq!(lines[lines.len() - 1], "- Second highlight", "got:\n{out}");
    assert!(
        blocks.contains(&"Work"),
        "section heading stands alone; got:\n{out}"
    );
    assert!(
        blocks.contains(&"Correspondent - Difference Engine Society"),
        "second entry is its own block; got:\n{out}"
    );
}

/// Given an entry `classic` lays out as a title with a right-aligned
/// date range, when rendered as text, then the title precedes the date
/// on one line.
#[test]
fn classic_reads_title_before_date_range() {
    let out = render_text("classic", &two_entry_resume());
    assert!(
        out.lines()
            .any(|l| l == "Programmer, Analytical Engines Ltd 1843-01-01 - 1852-11-27"),
        "title and date range must share one left-to-right line; got:\n{out}"
    );
}

/// Shared golden-file workflow: compile `theme_name` as text against the
/// named fixture, normalize the extracted text, and compare against
/// (or rewrite, under `UPDATE_GOLDEN`) the committed golden.
///
/// `required_name` is a substring the extracted text must contain
/// before we'll consider writing a golden from it — a cheap sanity
/// check that the extractor hasn't degenerated and we're not about to
/// freeze garbage.
fn run_golden(theme_name: &str, fixture: &str, golden: &str, required_name: &str) {
    let fixture_path = crate_path(fixture);
    let fixture_bytes = fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", fixture_path.display()));
    let data: Value = serde_json::from_slice(&fixture_bytes)
        .unwrap_or_else(|e| panic!("parse fixture {}: {e}", fixture_path.display()));

    let theme = ferrocv::find_theme(theme_name)
        .unwrap_or_else(|| panic!("theme `{theme_name}` must be registered in THEMES"));

    let raw = ferrocv::compile_text(theme, &data).unwrap_or_else(|e| {
        panic!(
            "{theme_name} must compile against {}: {e}",
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
        "{theme_name} text golden mismatch for {}. \
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
