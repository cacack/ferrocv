//! Negative unit tests for the `ferrocv::validate_value` library API.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #3, every class of invalid
//! input we claim to catch has a dedicated test asserting (a) we reject
//! it and (b) the diagnostic is useful — i.e. it carries the JSON
//! Pointer to the offending location and a non-empty message.
//!
//! These tests call the public library API directly, bypassing the
//! CLI. They do not assert on exact `jsonschema` error strings (those
//! change across crate versions); they use substring checks.

use std::path::PathBuf;

use ferrocv::{ValidationError, validate_value};
use serde_json::Value;

/// Load and parse a fixture JSON file by filename stem.
fn load_fixture(name: &str) -> Value {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {name}.json must be readable: {err}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("fixture {name}.json must parse as JSON: {err}"))
}

/// Assert that `validate_value` rejects `fixture_name` and at least one
/// reported `ValidationError` has `path == expected_path`. Returns the
/// matching error so callers can make further assertions on it.
fn expect_error_at(fixture_name: &str, expected_path: &str) -> ValidationError {
    let value = load_fixture(fixture_name);
    let errors = match validate_value(&value) {
        Ok(()) => panic!("expected fixture {fixture_name} to fail validation but it was accepted"),
        Err(e) => e,
    };
    assert!(
        !errors.is_empty(),
        "validate_value returned Err with empty error list for {fixture_name}",
    );
    errors
        .iter()
        .find(|e| e.path == expected_path)
        .cloned()
        .unwrap_or_else(|| {
            let paths: Vec<&str> = errors.iter().map(|e| e.path.as_str()).collect();
            panic!("no error at path `{expected_path}` for {fixture_name}; got paths: {paths:?}")
        })
}

/// Assert the Display output is non-empty and contains at least the
/// JSON pointer path (so diagnostic consumers can locate the error).
fn assert_display_useful(err: &ValidationError) {
    let rendered = err.to_string();
    assert!(
        !rendered.is_empty(),
        "ValidationError Display must not be empty",
    );
    assert!(
        rendered.contains(&err.path),
        "ValidationError Display `{rendered}` must contain its path `{}`",
        err.path,
    );
    assert!(
        !err.message.is_empty(),
        "ValidationError message must not be empty (path {})",
        err.path,
    );
}

#[test]
fn wrong_type_email_is_reported_at_basics_email() {
    let err = expect_error_at("invalid_wrong_type_email", "/basics/email");
    assert_display_useful(&err);
    // Schema keyword that fails here is `type`. Message should mention
    // the offending value (12345) or the expected type ("string").
    let rendered = err.to_string();
    assert!(
        rendered.contains("string") || rendered.contains("12345"),
        "expected diagnostic to mention expected type or offending value; got: {rendered}",
    );
}

#[test]
fn bad_email_format_is_reported_at_basics_email() {
    let err = expect_error_at("invalid_bad_email_format", "/basics/email");
    assert_display_useful(&err);
    let rendered = err.to_string();
    assert!(
        rendered.contains("email") || rendered.contains("not-an-email"),
        "expected diagnostic to mention `email` format or offending value; got: {rendered}",
    );
}

#[test]
fn bad_url_format_is_reported_at_basics_url() {
    let err = expect_error_at("invalid_bad_url_format", "/basics/url");
    assert_display_useful(&err);
    let rendered = err.to_string();
    // The schema uses format: "uri"; the jsonschema message echoes that
    // keyword or the offending value.
    assert!(
        rendered.contains("uri") || rendered.contains("not a url"),
        "expected diagnostic to mention `uri` format or offending value; got: {rendered}",
    );
}

#[test]
fn bad_date_format_is_reported_at_work_start_date() {
    let err = expect_error_at("invalid_bad_date_format", "/work/0/startDate");
    assert_display_useful(&err);
    let rendered = err.to_string();
    // iso8601 is enforced via `pattern`; messages usually mention
    // "pattern" or echo the bad value.
    assert!(
        rendered.contains("pattern")
            || rendered.contains("match")
            || rendered.contains("not-a-date"),
        "expected diagnostic to mention pattern failure or offending value; got: {rendered}",
    );
}

#[test]
fn array_item_type_is_reported_at_skills_keywords() {
    let err = expect_error_at("invalid_array_item_type", "/skills/0/keywords");
    assert_display_useful(&err);
    let rendered = err.to_string();
    assert!(
        rendered.contains("array") || rendered.contains("should-be-an-array"),
        "expected diagnostic to mention array type or offending value; got: {rendered}",
    );
}

#[test]
fn nested_wrong_type_is_reported_at_work_name() {
    let err = expect_error_at("invalid_nested_wrong_type", "/work/0/name");
    assert_display_useful(&err);
    let rendered = err.to_string();
    assert!(
        rendered.contains("string") || rendered.contains("42"),
        "expected diagnostic to mention expected type or offending value; got: {rendered}",
    );
    // Sanity: the path must traverse through a numeric array index, so
    // consumers can locate the failing entry in a multi-entry `work`.
    assert!(
        err.path.starts_with("/work/0/"),
        "path `{}` must anchor at `/work/0/`",
        err.path,
    );
}
