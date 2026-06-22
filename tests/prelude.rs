//! Tests for the shared native-theme prelude
//! (`assets/themes/_prelude/lib.typ`).
//!
//! The prelude is the data-access half of CONSTITUTION §4's
//! `render(data) -> content` native-theme contract. `text-minimal` and
//! `html-minimal` already exercise it indirectly through their committed
//! goldens, but those pin *layout*; this file pins the *contract* —
//! the optional-field helpers (`opt`, `nz`, `join_present`,
//! `date_range`) and the normalized section accessor (`items`) — on its
//! own so a regression shows up here rather than as a confusing golden
//! diff in two unrelated themes.
//!
//! Doctrine §4 (no mocking Typst): we drive the real embedded compiler
//! via [`ferrocv::compile_text_resolved`]. The prelude is served to the
//! World as a file in an [`ferrocv::OwnedTheme`], exactly as the bundled
//! native themes serve it — the test entrypoint `#import`s it by the
//! same absolute virtual path the real themes use.

use std::fs;
use std::path::PathBuf;

use ferrocv::{OwnedTheme, PRELUDE_PATH, ResolvedTheme, compile_text_resolved};
use serde_json::json;

/// Virtual path for the throwaway test entrypoint.
const ENTRY_VPATH: &str = "/themes/prelude-test/resume.typ";

/// Build a [`ResolvedTheme::Owned`] whose files are the real prelude
/// bytes plus an entrypoint that imports the prelude and then runs
/// `body`, then render it to text against `data`.
///
/// The import line is generated from the exported [`ferrocv::PRELUDE_PATH`]
/// — the same constant the bundled themes register the prelude under —
/// so the test and production can never drift on the prelude's location.
fn render_with_prelude(body: &str, data: &serde_json::Value) -> String {
    let prelude_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/themes/_prelude/lib.typ");
    let prelude_bytes = fs::read(&prelude_path).unwrap_or_else(|e| {
        panic!(
            "prelude must be readable at {}: {e}",
            prelude_path.display()
        )
    });

    let entry_src = format!("#import \"{PRELUDE_PATH}\": *\n#set page(margin: 1in)\n{body}");

    let theme = ResolvedTheme::Owned(OwnedTheme {
        name: "prelude-test".to_owned(),
        files: vec![
            (PRELUDE_PATH.to_owned(), prelude_bytes),
            (ENTRY_VPATH.to_owned(), entry_src.into_bytes()),
        ],
        entrypoint: ENTRY_VPATH.to_owned(),
    });

    compile_text_resolved(&theme, data).expect("prelude-test theme must compile to text")
}

/// The four helpers behave as documented: `nz` collapses empty/none,
/// `opt` reads present keys and tolerates wrong/missing shapes,
/// `join_present` drops blanks (and yields `""`, not `none`, when every
/// part is absent), and `date_range` covers all four bound combinations.
#[test]
fn helpers_behave_as_documented() {
    // Each line emits a marker only when the helper behaves correctly,
    // so a regression flips a marker (or removes it) in the extracted
    // text. `date_range` has four branches — all are exercised here so a
    // regression surfaces in this test rather than as a golden diff.
    let body = r#"
#[#("DR-START:" + date_range((startDate: "2020")))]
#linebreak()
#[#("DR-BOTH:" + date_range((startDate: "2019", endDate: "2023")))]
#linebreak()
#[#("DR-END:" + date_range((endDate: "2021")))]
#linebreak()
#[#(if date_range((:)) == none { "DR-NONE-OK" } else { "DR-NONE-BUG" })]
#linebreak()

#[#("JP:" + join_present(("a", none, "", "b"), ", "))]
#linebreak()
#[#("JP-EMPTY:[" + join_present((none, ""), " - ") + "]")]
#linebreak()

#[#(if nz("") == none and nz("x") == "x" { "NZ-OK" } else { "NZ-BUG" })]
#linebreak()

#[#(if opt((k: "v"), "k") == "v" and opt((:), "missing") == none and opt("not-a-dict", "k") == none { "OPT-OK" } else { "OPT-BUG" })]
"#;
    let text = render_with_prelude(body, &json!({}));

    assert!(
        text.contains("DR-START:2020 - Present"),
        "date_range start-only must append ' - Present'; got:\n{text}"
    );
    assert!(
        text.contains("DR-BOTH:2019 - 2023"),
        "date_range with both bounds must join them; got:\n{text}"
    );
    assert!(
        text.contains("DR-END:2021"),
        "date_range end-only must yield the end date; got:\n{text}"
    );
    assert!(
        text.contains("DR-NONE-OK"),
        "date_range with neither bound must return none; got:\n{text}"
    );
    assert!(
        text.contains("JP:a, b"),
        "join_present must drop none/empty parts; got:\n{text}"
    );
    assert!(
        text.contains("JP-EMPTY:[]"),
        "join_present must yield the empty string (not none) when all parts absent; got:\n{text}"
    );
    assert!(
        text.contains("NZ-OK"),
        "nz must collapse empty/none; got:\n{text}"
    );
    assert!(
        text.contains("OPT-OK"),
        "opt must read present keys and tolerate missing keys and non-dict inputs; got:\n{text}"
    );
}

/// `items` yields the array for a present non-empty key and an empty
/// array for missing keys, non-array values, and empty arrays — so a
/// `for` over it is safe on any schema-valid (or even malformed)
/// document without a guard.
#[test]
fn items_normalizes_section_access() {
    let body = r#"
#let data = json("/resume.json")

#[#("PRESENT:" + str(items(data, "work").len()))]
#linebreak()
#[#("MISSING:" + str(items(data, "education").len()))]
#linebreak()
#[#("NONARRAY:" + str(items(data, "basics").len()))]
#linebreak()
#[#("EMPTY:" + str(items(data, "skills").len()))]
#linebreak()

#for w in items(data, "work") {
  let n = nz(opt(w, "name"))
  if n != none { [#("W:" + n)]; linebreak() }
}
#for _ in items(data, "education") { [SHOULD-NOT-APPEAR]; linebreak() }
"#;
    // `basics` is a dictionary (non-array), `skills` is an empty array,
    // `education` is absent — all three must normalize to length 0.
    let data = json!({
        "basics": { "name": "Ada" },
        "skills": [],
        "work": [ { "name": "Acme" }, { "name": "Beta" } ],
    });
    let text = render_with_prelude(body, &data);

    assert!(
        text.contains("PRESENT:2"),
        "present array length; got:\n{text}"
    );
    assert!(
        text.contains("MISSING:0"),
        "absent key -> empty; got:\n{text}"
    );
    assert!(
        text.contains("NONARRAY:0"),
        "non-array value -> empty; got:\n{text}"
    );
    assert!(
        text.contains("EMPTY:0"),
        "empty array stays empty; got:\n{text}"
    );
    assert!(
        text.contains("W:Acme") && text.contains("W:Beta"),
        "iterates present array; got:\n{text}"
    );
    assert!(
        !text.contains("SHOULD-NOT-APPEAR"),
        "iterating a missing section must produce no output; got:\n{text}"
    );
}
