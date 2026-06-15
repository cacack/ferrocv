//! Exhaustive re-validation of the projection stage (issue #150).
//!
//! `CONSTITUTION.md` §7 promises the projection stage emits a **derived
//! document that is itself still valid JSON Resume**, which then flows
//! into the unchanged render pipeline. These tests lock that guarantee
//! in (testing doctrine §3) at the library level via the public API:
//! every filter — and every combination — is projected from a real
//! schema-valid master and the result is re-validated against the
//! embedded schema.
//!
//! The CLI-surface round-trips live in `tests/tailor_cli.rs`; the
//! transform's own unit tests live in `src/project.rs`. This file is the
//! "derived output always re-validates" property, exercised through
//! `ferrocv::project` + `ferrocv::validate_value`.

use std::path::PathBuf;

use ferrocv::{ProjectionError, ProjectionSpec, RedactSet, project, validate_value};
use serde_json::Value;

/// Load a fixture document by filename stem (no extension).
fn fixture(name: &str) -> Value {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{name}.json"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read fixture {name}: {e}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("fixture {name} parses as JSON: {e}"))
}

/// Build a [`ProjectionSpec`] by field assignment.
///
/// `ProjectionSpec` is `#[non_exhaustive]`, so an out-of-crate caller (an
/// integration test is its own crate) cannot use a struct literal — it
/// must start from `default()` and assign fields. This helper centralizes
/// that so the matrix below reads as data.
fn spec(
    audience: Option<&str>,
    since: Option<&str>,
    max_bullets: Option<usize>,
    redact: bool,
) -> ProjectionSpec {
    let mut s = ProjectionSpec::default();
    s.audience = audience.map(str::to_owned);
    s.since = since.map(str::to_owned);
    s.max_bullets = max_bullets;
    s.redact = redact.then_some(RedactSet::Pii);
    s
}

/// Recursively check whether any object in `value` carries an `x-ferrocv`
/// key. Walks the parsed tree rather than substring-matching raw bytes, so
/// a field *value* containing the text can't false-pass and a nested key
/// can't be missed. (Mirrors the helper in `tests/tailor_cli.rs`.)
fn contains_x_ferrocv_key(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key("x-ferrocv") || map.values().any(contains_x_ferrocv_key)
        }
        Value::Array(items) => items.iter().any(contains_x_ferrocv_key),
        _ => false,
    }
}

#[test]
fn master_fixture_is_itself_valid() {
    // Sanity anchor: the whole "derived re-validates" property is only
    // meaningful if the master we project from is valid to begin with.
    let master = fixture("master_projection");
    assert!(
        validate_value(&master).is_ok(),
        "master_projection fixture must be valid JSON Resume"
    );
}

#[test]
fn every_filter_and_combination_revalidates() {
    let master = fixture("master_projection");

    // The full matrix: each filter alone, plus the all-filters cut. The
    // `master_projection` fixture tags content for `security` and
    // `leadership`; `archaeology` is an audience nothing is tagged for
    // (exercises the "only universal entries survive" path).
    let cases: &[(&str, ProjectionSpec)] = &[
        (
            "audience=security",
            spec(Some("security"), None, None, false),
        ),
        (
            "audience=leadership",
            spec(Some("leadership"), None, None, false),
        ),
        (
            "audience=archaeology (untagged)",
            spec(Some("archaeology"), None, None, false),
        ),
        ("since=2015", spec(None, Some("2015"), None, false)),
        ("max_bullets=0", spec(None, None, Some(0), false)),
        ("max_bullets=2", spec(None, None, Some(2), false)),
        ("redact=pii", spec(None, None, None, true)),
        (
            "all-filters-combined",
            spec(Some("security"), Some("2015"), Some(2), true),
        ),
    ];

    for (label, s) in cases {
        let derived =
            project(&master, s).unwrap_or_else(|e| panic!("[{label}] project failed: {e}"));
        if let Err(errors) = validate_value(&derived) {
            panic!("[{label}] derived document failed re-validation: {errors:?}");
        }
    }
}

#[test]
fn audience_derived_document_is_clean_and_revalidates() {
    // The headline §7/#150 property for the curated path: after audience
    // selection the derived document carries no `x-ferrocv` key anywhere
    // (the tags were consumed and stripped) *and* re-validates.
    let master = fixture("master_projection");

    for audience in ["security", "leadership", "archaeology"] {
        let derived = project(&master, &spec(Some(audience), None, None, false))
            .unwrap_or_else(|e| panic!("[{audience}] project failed: {e}"));
        assert!(
            !contains_x_ferrocv_key(&derived),
            "[{audience}] derived document must not contain any x-ferrocv key"
        );
        assert!(
            validate_value(&derived).is_ok(),
            "[{audience}] derived document must re-validate"
        );
    }
}

#[test]
fn malformed_since_is_a_spec_error_with_no_document() {
    // Negative: a malformed `--since` value is rejected as a spec error
    // (no half-projected document is produced).
    let master = fixture("master_projection");
    let result = project(&master, &spec(None, Some("banana"), None, false));
    assert!(
        matches!(result, Err(ProjectionError::InvalidSince(ref v)) if v == "banana"),
        "expected InvalidSince(\"banana\"), got {result:?}"
    );
}

#[test]
fn audience_highlights_tag_mismatch_is_a_document_error_with_no_document() {
    // Negative: a master whose `x-ferrocv.highlights` length differs from
    // its `highlights` length is a document defect — projection errors and
    // emits no derived document rather than shipping a misattributed cut.
    let master = fixture("master_audience_mismatch");
    let result = project(&master, &spec(Some("security"), None, None, false));
    assert!(
        matches!(result, Err(ProjectionError::HighlightsTagMismatch { .. })),
        "expected HighlightsTagMismatch, got {result:?}"
    );
}
