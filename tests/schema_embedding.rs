//! Sanity checks that the JSON Resume schema is correctly embedded at
//! compile time. These tests deliberately do not compile the file as a
//! JSON Schema; that is the job of issue #10 (`ferrocv validate`).

use ferrocv::{JSON_RESUME_SCHEMA, JSON_RESUME_SCHEMA_VERSION};

#[test]
fn schema_is_non_empty() {
    assert!(
        !JSON_RESUME_SCHEMA.is_empty(),
        "embedded JSON Resume schema must not be empty",
    );
}

#[test]
fn schema_parses_as_json() {
    serde_json::from_str::<serde_json::Value>(JSON_RESUME_SCHEMA)
        .expect("embedded JSON Resume schema must parse as JSON");
}

#[test]
fn schema_has_expected_basics_section() {
    let value: serde_json::Value = serde_json::from_str(JSON_RESUME_SCHEMA)
        .expect("embedded JSON Resume schema must parse as JSON");
    let basics = value
        .get("properties")
        .and_then(|properties| properties.get("basics"))
        .expect("schema must define properties.basics");
    assert!(
        basics.is_object(),
        "properties.basics must be a JSON object, got: {basics}",
    );
}

#[test]
fn version_constant_matches_expected() {
    assert_eq!(JSON_RESUME_SCHEMA_VERSION, "1.0.0");
}
