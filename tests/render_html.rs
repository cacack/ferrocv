//! Loose-assertion well-formedness tests for HTML output.
//!
//! Enforces CONSTITUTION testing doctrine §2 ("every theme has a
//! golden-file test") for the HTML path, but with a deliberate
//! relaxation: **no byte-exact golden files**. Typst's HTML export
//! is explicitly labelled experimental upstream and its output shape
//! is guaranteed to shift across Typst minor bumps. A byte-exact
//! golden would produce noise PRs on every Typst version bump
//! without catching any real regression.
//!
//! Instead, we assert on:
//! - Structural well-formedness (starts with `<!DOCTYPE html>`,
//!   ends with `</html>`, contains a `<body>...</body>` pair in
//!   the right order).
//! - Content presence (the fixture's name string makes it through).
//! - External-reference negation (no `src=`, no `href=` except
//!   anchor links, no `<link rel`, no `@font-face`, no `url(`) —
//!   guards against a future Typst bump silently turning on
//!   external assets, which would break CONSTITUTION §6.1's
//!   "single-file output" promise.
//!
//! Runs the real embedded Typst compiler via
//! [`ferrocv::compile_html`] (doctrine §4: no mocking Typst).
//!
//! See `research/44-html-viability.md` §2 for the rationale behind
//! the loose-assertion choice.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

#[test]
fn html_compiles_full_fixture_to_well_formed_html() {
    let data = read_fixture("tests/fixtures/render_full.json");
    let theme = ferrocv::find_theme("text-minimal").expect("text-minimal must be registered");

    let html = ferrocv::compile_html(theme, &data)
        .expect("text-minimal must compile render_full.json to HTML");

    // Structural well-formedness.
    assert!(
        html.starts_with("<!DOCTYPE html>"),
        "HTML must begin with <!DOCTYPE html>; got first 80 chars:\n{}",
        html.chars().take(80).collect::<String>(),
    );
    assert!(
        html.trim_end().ends_with("</html>"),
        "HTML must end with </html> (after trimming); got last 80 chars:\n{}",
        html.chars()
            .rev()
            .take(80)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>(),
    );

    // `<body>` and `</body>` must both exist, and the opening tag
    // must appear before the closing tag. Substring search rather
    // than HTML parsing — loose by design.
    let body_open = html
        .find("<body>")
        .expect("HTML must contain a <body> opening tag");
    let body_close = html
        .find("</body>")
        .expect("HTML must contain a </body> closing tag");
    assert!(
        body_open < body_close,
        "<body> must appear before </body>; got body_open={body_open}, body_close={body_close}",
    );

    // Content presence: the rendered name must survive into the HTML.
    assert!(
        html.contains("Ada Lovelace"),
        "HTML must contain the fixture's name 'Ada Lovelace'; got {} bytes",
        html.len(),
    );

    // External-reference negation. A regression that silently turns
    // on external CSS, fonts, or images would break the single-file
    // guarantee (see CONSTITUTION §6.1 and research/44-html-viability.md
    // §5). Fragment-only `href="#..."` anchors are allowed.
    assert!(
        !html.contains("src=\""),
        "HTML must not contain any `src=\"...\"` references; found one in:\n{}",
        preview(&html, "src=\""),
    );
    for bad in ["<link rel", "@font-face", "url("] {
        assert!(
            !html.contains(bad),
            "HTML must not contain `{bad}`; found one in:\n{}",
            preview(&html, bad),
        );
    }
    // Scan every `href="..."` and reject any that is not a fragment
    // anchor. Iterative search keeps the assertion honest without
    // pulling in an HTML parser.
    let mut rest = html.as_str();
    while let Some(idx) = rest.find("href=\"") {
        let after = &rest[idx + 6..];
        // The first char after `href="` must be `#` for the link to
        // count as a fragment-only anchor.
        let first_char = after.chars().next();
        assert!(
            matches!(first_char, Some('#')),
            "HTML href must be a fragment anchor (`#...`) only; found href starting with {first_char:?} in:\n{}",
            preview(rest, "href=\""),
        );
        rest = &after[1..];
    }
}

#[test]
fn html_compiles_sparse_fixture_without_errors() {
    let data = read_fixture("tests/fixtures/render_sparse.json");
    let theme = ferrocv::find_theme("text-minimal").expect("text-minimal must be registered");

    let html = ferrocv::compile_html(theme, &data)
        .expect("text-minimal must compile render_sparse.json to HTML");

    assert!(
        html.contains("Grace Hopper"),
        "HTML must contain the sparse fixture's name 'Grace Hopper'; got {} bytes",
        html.len(),
    );
}

/// Resolve a path relative to the crate root (where `Cargo.toml`
/// lives). Duplicated from `tests/render_text.rs::crate_path` per
/// CONSTITUTION §5 (share on the third caller, not the second).
fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Read and parse a JSON Resume fixture from the test tree.
fn read_fixture(relative: &str) -> Value {
    let path = crate_path(relative);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("parse fixture {}: {e}", path.display()))
}

/// Return a short preview of `haystack` around the first occurrence
/// of `needle`, for assertion error messages. Bounded to 120 chars
/// so failing tests don't dump the entire HTML document.
fn preview(haystack: &str, needle: &str) -> String {
    match haystack.find(needle) {
        Some(idx) => {
            let start = idx.saturating_sub(40);
            let end = (idx + needle.len() + 40).min(haystack.len());
            haystack[start..end].to_string()
        }
        None => "<not found>".to_string(),
    }
}
