//! Guard tests for the user-facing example master shipped under
//! `examples/master.resume.json` and documented in `docs/tailoring.md`.
//!
//! These spawn the real built binary via `assert_cmd` and assert on
//! observable behavior only. Their job is to keep the docs honest: if
//! the example file or the projection behavior drifts from what the
//! tailoring guide claims, one of these fails rather than the guide
//! silently going stale.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #1, CLI-visible behavior gets
//! a scenario test. The example master is part of the #151 docs
//! deliverable; these tests pin the exact keep/drop the guide documents.

use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

/// Absolute path to the shipped example master.
fn example_master() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path.push("master.resume.json");
    path
}

/// Build a `Command` for the `ferrocv` binary under test.
fn ferrocv() -> Command {
    Command::cargo_bin("ferrocv").expect("binary `ferrocv` must be built")
}

/// Names of the `work` entries in a document, in order.
fn work_names(doc: &Value) -> Vec<String> {
    doc["work"]
        .as_array()
        .expect("work is an array")
        .iter()
        .map(|e| {
            e["name"]
                .as_str()
                .expect("work entry has a name")
                .to_owned()
        })
        .collect()
}

/// Highlights of the named `work` entry, in order.
fn work_highlights(doc: &Value, name: &str) -> Vec<String> {
    doc["work"]
        .as_array()
        .expect("work is an array")
        .iter()
        .find(|e| e["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("work entry `{name}` is present"))
        .get("highlights")
        .and_then(Value::as_array)
        .map(|hs| {
            hs.iter()
                .map(|h| h.as_str().expect("highlight is a string").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Values of the named string field across an array section's entries.
fn names_in(doc: &Value, section: &str, key: &str) -> Vec<String> {
    doc[section]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| e[key].as_str().expect("entry has the key").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn example_master_is_valid_json_resume() {
    // The guide tells users to copy this file as a starting point, so it
    // must pass `validate` (exit 0) like any master would.
    ferrocv()
        .arg("validate")
        .arg(example_master())
        .assert()
        .success();
}

/// Run `tailor --audience <name>` on the example and parse the derived doc.
fn tailor_audience(name: &str) -> Value {
    let assert = ferrocv()
        .arg("tailor")
        .arg(example_master())
        .arg("--audience")
        .arg(name)
        .assert()
        .success();
    serde_json::from_slice(&assert.get_output().stdout).expect("derived stdout is valid JSON")
}

#[test]
fn security_cut_matches_the_guide() {
    let doc = tailor_audience("security");

    // All three work entries survive: Northwind is tagged for security,
    // Cobalt and Riverstone are untagged (universal).
    assert_eq!(
        work_names(&doc),
        [
            "Northwind Platform",
            "Cobalt Analytics",
            "Riverstone Software"
        ]
    );

    // Northwind's leadership-only bullet drops; the security and
    // universal bullets stay (matching the guide's worked-example table).
    let northwind = work_highlights(&doc, "Northwind Platform");
    assert!(
        !northwind
            .iter()
            .any(|h| h.contains("Grew the platform team")),
        "leadership-only bullet must be dropped from the security cut"
    );
    assert!(
        northwind
            .iter()
            .any(|h| h.contains("short-lived workload credentials"))
    );
    assert!(northwind.iter().any(|h| h.contains("SOC 2")));

    // The security-tagged volunteer entry and skill are kept.
    assert!(
        names_in(&doc, "volunteer", "organization").contains(&"OWASP Local Chapter".to_owned())
    );
    assert!(names_in(&doc, "skills", "name").contains(&"Security".to_owned()));
}

#[test]
fn leadership_cut_drops_security_only_entries() {
    let doc = tailor_audience("leadership");

    // The security-only volunteer entry and skill disappear from the
    // leadership cut; the untagged ones remain.
    let volunteers = names_in(&doc, "volunteer", "organization");
    assert!(!volunteers.contains(&"OWASP Local Chapter".to_owned()));
    assert!(volunteers.contains(&"Code Mentors".to_owned()));

    let skills = names_in(&doc, "skills", "name");
    assert!(!skills.contains(&"Security".to_owned()));
    assert!(skills.contains(&"Languages".to_owned()));
}

#[test]
fn derived_cut_strips_all_x_ferrocv_metadata() {
    // The consumed control metadata must not travel into the cut.
    let doc = tailor_audience("security");
    let serialized = serde_json::to_string(&doc).expect("re-serialize derived doc");
    assert!(
        !serialized.contains("x-ferrocv"),
        "no x-ferrocv key may survive in the derived document"
    );
}
