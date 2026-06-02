//! Scenario-style black-box tests for the `ferrocv tailor` subcommand.
//!
//! These tests spawn the real built binary via `assert_cmd` and assert
//! on observable behavior only: exit code, stdout (the derived JSON
//! Resume), stderr (diagnostics only), and output files. They do not
//! call into the library API — `src/project.rs`'s unit tests cover the
//! transform directly.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #1, every CLI-visible
//! behavior gets a scenario test, written before the implementation.
//! `tailor` is the standalone projection surface from ADR 0005; this
//! issue (#148) implements its mechanical filters (`--since`,
//! `--max-bullets`, `--redact`). The exit-code contract mirrors the
//! other subcommands:
//! - `0` — projected; derived JSON Resume written to `-o`/stdout
//! - `1` — master parsed but failed schema validation
//! - `2` — usage error (bad flag value), IO error, or parse error

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

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

/// Parse the master fixture from disk for structural comparisons.
fn master() -> Value {
    let bytes = std::fs::read(fixture("master_projection")).expect("read master fixture");
    serde_json::from_slice(&bytes).expect("master fixture parses as JSON")
}

/// Names of the `work` entries in a derived document, in order.
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

#[test]
fn tailor_no_flags_is_identity() {
    // With no projection flags the derived document must be structurally
    // equal to the master (projection is opt-in and inert). Compared as
    // serde_json::Value, so key ordering and whitespace don't matter.
    let assert = ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");
    assert_eq!(doc, master(), "no-flags tailor must equal the master");
}

#[test]
fn tailor_since_drops_old_keeps_recent_and_ongoing() {
    // --since 2015: drop work entries whose endDate sorts before 2015,
    // keep recent ones and any ongoing entry (no endDate).
    let assert = ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--since")
        .arg("2015")
        .assert()
        .success();
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");
    let names = work_names(&doc);
    assert!(names.contains(&"Current Corp".to_owned()), "ongoing kept");
    assert!(names.contains(&"Mid Corp".to_owned()), "recent kept");
    assert!(!names.contains(&"Old Corp".to_owned()), "old dropped");
    assert_eq!(names.len(), 2, "exactly two work entries survive");
}

#[test]
fn tailor_max_bullets_caps_highlights() {
    // --max-bullets 2: every highlights array capped to its first 2
    // entries, by position.
    let assert = ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--max-bullets")
        .arg("2")
        .assert()
        .success();
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");

    for entry in doc["work"].as_array().unwrap() {
        if let Some(h) = entry.get("highlights").and_then(Value::as_array) {
            assert!(h.len() <= 2, "work highlights capped at 2");
        }
    }
    // Current Corp had 4 highlights; first 2 by position survive.
    let current = doc["work"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "Current Corp")
        .expect("Current Corp present");
    let highlights = current["highlights"].as_array().unwrap();
    assert_eq!(highlights.len(), 2);
    assert_eq!(highlights[0], "Led the platform rewrite");
    assert_eq!(highlights[1], "Mentored eight engineers");

    // The cap also applies to volunteer highlights (any bare-string
    // highlights array), not just work.
    let vol = &doc["volunteer"].as_array().unwrap()[0];
    assert_eq!(vol["highlights"].as_array().unwrap().len(), 2);
}

#[test]
fn tailor_redact_pii_removes_fields_keeps_name() {
    let assert = ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--redact")
        .arg("pii")
        .assert()
        .success();
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");
    let basics = &doc["basics"];
    assert!(basics.get("location").is_none(), "location redacted");
    assert!(basics.get("phone").is_none(), "phone redacted");
    assert!(basics.get("email").is_none(), "email redacted");
    // Non-PII identity fields are kept.
    assert_eq!(basics["name"], "Grace Hopper", "name kept");
    assert!(basics.get("label").is_some(), "label kept");
    assert!(basics.get("summary").is_some(), "summary kept");
    assert!(basics.get("url").is_some(), "url kept");
}

#[test]
fn tailor_filters_compose() {
    // All three filters in one invocation; verifies they compose.
    let assert = ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--since")
        .arg("2015")
        .arg("--max-bullets")
        .arg("1")
        .arg("--redact")
        .arg("pii")
        .assert()
        .success();
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");
    assert_eq!(work_names(&doc).len(), 2, "since applied");
    assert!(doc["basics"].get("email").is_none(), "redact applied");
    for entry in doc["work"].as_array().unwrap() {
        if let Some(h) = entry.get("highlights").and_then(Value::as_array) {
            assert!(h.len() <= 1, "max-bullets applied");
        }
    }
}

#[test]
fn tailor_preserves_x_ferrocv_tags() {
    // Mechanical filters (#148) neither consume nor strip x-ferrocv;
    // curated --audience selection (#149) owns that. The tag rides
    // through untouched.
    let assert = ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--since")
        .arg("2015")
        .assert()
        .success();
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");
    let current = doc["work"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "Current Corp")
        .expect("Current Corp present");
    assert!(
        current.get("x-ferrocv").is_some(),
        "x-ferrocv tags preserved by mechanical filters"
    );
}

#[test]
fn tailor_writes_to_output_file_and_keeps_stdout_clean() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("cut.json");
    ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--redact")
        .arg("pii")
        .arg("-o")
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(out.exists(), "derived file written");
    let bytes = std::fs::read(&out).expect("read derived file");
    let doc: Value = serde_json::from_slice(&bytes).expect("derived file is valid JSON");
    assert!(doc["basics"].get("email").is_none());
}

#[test]
fn tailor_derived_document_revalidates() {
    // ADR 0005 / doctrine: the derived document is itself valid JSON
    // Resume and flows back into the pipeline. Tailor to a file, then
    // validate that file with the same binary.
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("cut.json");
    ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--since")
        .arg("2015")
        .arg("--max-bullets")
        .arg("2")
        .arg("--redact")
        .arg("pii")
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    ferrocv()
        .arg("validate")
        .arg(&out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn tailor_reads_master_from_stdin() {
    let input = std::fs::read_to_string(fixture("master_projection")).expect("read master");
    let assert = ferrocv()
        .arg("tailor")
        .arg("--redact")
        .arg("pii")
        .write_stdin(input)
        .assert()
        .success();
    let doc: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout is valid JSON");
    assert!(doc["basics"].get("phone").is_none());
}

#[test]
fn tailor_rejects_invalid_master() {
    ferrocv()
        .arg("tailor")
        .arg(fixture("invalid_wrong_type_email"))
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        // A schema diagnostic must reach stderr — guards against a
        // refactor that silences validation errors or misroutes them.
        .stderr(predicate::str::contains("error"));
}

#[test]
fn tailor_rejects_malformed_since() {
    ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--since")
        .arg("banana")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty());
}

#[test]
fn tailor_malformed_since_beats_invalid_master() {
    // A malformed projection flag is a usage error (exit 2) and must win
    // over a schema-invalid master (exit 1): bad CLI input is rejected
    // before the document is even validated.
    ferrocv()
        .arg("tailor")
        .arg(fixture("invalid_wrong_type_email"))
        .arg("--since")
        .arg("banana")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty());
}

#[test]
fn tailor_rejects_unknown_redact_value() {
    // --redact is a fixed vocabulary (clap ValueEnum); an unknown value
    // is a clap usage error (exit 2).
    ferrocv()
        .arg("tailor")
        .arg(fixture("master_projection"))
        .arg("--redact")
        .arg("everything")
        .assert()
        .code(2);
}
