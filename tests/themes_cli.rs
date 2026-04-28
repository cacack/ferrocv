//! Scenario-style black-box tests for the `ferrocv themes` subcommand.
//!
//! These tests spawn the real built binary via `assert_cmd` and assert
//! on observable behavior only: exit code, stdout, stderr.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #1, every CLI-visible
//! behavior gets a scenario test. The exit-code contract under test:
//! 0 = success. The exact-stdout assertion is deliberate — it locks
//! in the machine-readable output contract so any accidental
//! decoration (headers, blank lines, extra whitespace) fails loudly.

use assert_cmd::Command;
use predicates::prelude::*;

/// Build a `Command` for the `ferrocv` binary under test.
fn ferrocv() -> Command {
    Command::cargo_bin("ferrocv").expect("binary `ferrocv` must be built")
}

#[test]
fn themes_list_prints_sorted_names() {
    ferrocv()
        .arg("themes")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::eq(
            "basic-resume\nfantastic-cv\nhtml-minimal\nmodern-cv\ntext-minimal\ntypst-jsonresume-cv\n",
        ))
        .stderr(predicate::str::is_empty());
}
