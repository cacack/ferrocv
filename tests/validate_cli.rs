//! Scenario-style black-box tests for the `ferrocv validate` subcommand.
//!
//! These tests spawn the real built binary via `assert_cmd` and assert
//! on observable behavior only: exit code, stdout, stderr. They do not
//! call into the library API — see `validate_unit.rs` for that.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #1, every CLI-visible
//! behavior gets a scenario test. The exit-code contract under test:
//! 0 = valid, 1 = schema-invalid, 2 = IO / parse error.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

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

#[test]
fn validate_accepts_minimal_resume() {
    ferrocv()
        .arg("validate")
        .arg(fixture("valid_minimal"))
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn validate_accepts_full_resume() {
    ferrocv()
        .arg("validate")
        .arg(fixture("valid_full"))
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn validate_rejects_wrong_type_email() {
    ferrocv()
        .arg("validate")
        .arg(fixture("invalid_wrong_type_email"))
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("/basics/email"));
}

#[test]
fn validate_reports_parse_error_with_exit_code_two() {
    ferrocv()
        .arg("validate")
        .arg(fixture("malformed"))
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        // The parse diagnostic comes from serde_json; we only assert
        // that the error-prefixed line is present and mentions either
        // "line" (serde_json's default wording) or "column" so the
        // test survives minor upstream wording changes.
        .stderr(predicate::str::contains("error:"))
        .stderr(predicate::str::contains("line").or(predicate::str::contains("column")));
}

#[test]
fn validate_reports_missing_file_with_exit_code_two() {
    let missing = "/nonexistent/path/that/does/not/exist.json";
    ferrocv()
        .arg("validate")
        .arg(missing)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(missing));
}

#[test]
fn validate_accepts_valid_json_on_stdin() {
    let input = std::fs::read_to_string(fixture("valid_minimal"))
        .expect("fixture `valid_minimal.json` must be readable");
    ferrocv()
        .arg("validate")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn validate_rejects_invalid_json_on_stdin() {
    let input = std::fs::read_to_string(fixture("invalid_bad_email_format"))
        .expect("fixture `invalid_bad_email_format.json` must be readable");
    ferrocv()
        .arg("validate")
        .write_stdin(input)
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("/basics/email"));
}
