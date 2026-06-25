//! Scenario-style black-box tests for `ferrocv themes new` (issue #181).
//!
//! These spawn the real built binary via `assert_cmd` and assert on
//! observable behavior only: exit code, emitted files, and — the
//! headline acceptance criterion — that the scaffolded theme renders
//! BOTH golden fixtures (`render_full.json`, `render_sparse.json`) out
//! of the box, with no edits, through the same binary.
//!
//! Per `CONSTITUTION.md` §Testing doctrine #1 (every CLI-visible
//! behavior has a scenario test) and #4 (no mocking Typst — the render
//! step drives the real embedded compiler). The exit-code contract:
//! - `0` — theme scaffolded / render succeeded
//! - `2` — invalid name, target already exists, or IO error
//!
//! The "renders out of the box" guarantee exercises the universal
//! prelude injection: the emitted `resume.typ` is a *local single-file*
//! theme, yet it `#import`s `/themes/_prelude/lib.typ`, which only
//! resolves because `FerrocvWorld` now serves the prelude in every
//! World (not just bundled themes). A regression there would surface as
//! a Typst "file not found" and fail the render assertions below.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// PDF magic bytes; every valid PDF stream starts with `%PDF-`.
const PDF_MAGIC: &[u8] = b"%PDF-";

/// Build a `Command` for the `ferrocv` binary under test.
fn ferrocv() -> Command {
    Command::cargo_bin("ferrocv").expect("binary `ferrocv` must be built")
}

/// Absolute path to a JSON fixture by filename stem (no extension).
fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(format!("{name}.json"));
    path
}

/// Render `fixture_stem` through the local-path `theme` to `out`, and
/// assert it produced a real PDF. Drives the real embedded Typst.
fn assert_renders(theme: &Path, fixture_stem: &str, out: &Path) {
    ferrocv()
        .arg("render")
        .arg(fixture(fixture_stem))
        .arg("--theme")
        .arg(theme)
        .arg("-o")
        .arg(out)
        .assert()
        .success();

    let bytes = fs::read(out).expect("render must have written the output file");
    assert!(
        bytes.starts_with(PDF_MAGIC),
        "scaffolded theme rendered `{fixture_stem}` to a non-PDF output: {:?}",
        &bytes[..bytes.len().min(8)],
    );
}

#[test]
fn scaffold_emits_files_and_renders_both_fixtures() {
    let tmp = TempDir::new().expect("create temp dir");

    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("mytheme")
        .arg("--out")
        .arg(tmp.path())
        .assert()
        .success();

    let theme_dir = tmp.path().join("mytheme");
    let resume_typ = theme_dir.join("resume.typ");
    assert!(resume_typ.is_file(), "scaffold must emit resume.typ");
    assert!(
        theme_dir.join("golden.txt").is_file(),
        "scaffold must emit a golden-test stub"
    );

    // The acceptance criterion: the generated theme renders both
    // fixtures with no edits, through the already-built binary.
    assert_renders(&resume_typ, "render_full", &tmp.path().join("full.pdf"));
    assert_renders(&resume_typ, "render_sparse", &tmp.path().join("sparse.pdf"));
}

#[test]
fn scaffold_defaults_out_to_current_directory() {
    // With no --out, the theme dir is created relative to the process
    // cwd. Run the binary with cwd set to a temp dir so the scaffold
    // lands there and we don't litter the repo.
    let tmp = TempDir::new().expect("create temp dir");

    ferrocv()
        .current_dir(tmp.path())
        .arg("themes")
        .arg("new")
        .arg("here")
        .assert()
        .success();

    assert!(
        tmp.path().join("here").join("resume.typ").is_file(),
        "scaffold without --out must write under the current directory"
    );
}

#[test]
fn scaffold_refuses_to_clobber_existing_target() {
    let tmp = TempDir::new().expect("create temp dir");

    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("dup")
        .arg("--out")
        .arg(tmp.path())
        .assert()
        .success();

    // Second run against the same target must refuse (exit 2) and
    // leave the existing theme untouched.
    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("dup")
        .arg("--out")
        .arg(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn scaffold_refuses_when_target_is_a_nondirectory() {
    // Exclusive `create_dir` must refuse even when a *file* (not a
    // directory) already occupies the target path — regression guard for
    // the refuse-if-exists / no-clobber contract.
    let tmp = TempDir::new().expect("create temp dir");
    let occupied = tmp.path().join("taken");
    fs::write(&occupied, b"not a theme").expect("seed a file at the target path");

    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("taken")
        .arg("--out")
        .arg(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));

    // The pre-existing file must be left untouched.
    assert_eq!(
        fs::read(&occupied).expect("file still present"),
        b"not a theme",
        "refused scaffold must not clobber the existing file"
    );
}

#[test]
fn scaffold_rejects_name_with_path_separator() {
    let tmp = TempDir::new().expect("create temp dir");

    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("a/b")
        .arg("--out")
        .arg(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid theme name"));
}

#[test]
fn scaffold_rejects_leading_hyphen_name() {
    // A name like `-x` would make the emitted `--theme -x/resume.typ`
    // value start with `-` (which clap parses as a flag), so the
    // validator rejects it. clap blocks a bare `themes new -x` itself,
    // so reach the validator the only way a user can: through `--`.
    let tmp = TempDir::new().expect("create temp dir");

    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("--out")
        .arg(tmp.path())
        .arg("--")
        .arg("-leading")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("must not start with"));

    assert!(
        !tmp.path().join("-leading").exists(),
        "a rejected name must not create a directory"
    );
}

#[test]
fn scaffold_rejects_dotdot_name() {
    let tmp = TempDir::new().expect("create temp dir");

    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("../escape")
        .arg("--out")
        .arg(tmp.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid theme name"));

    // The traversal target must not have been created.
    assert!(
        !tmp.path().parent().unwrap().join("escape").exists(),
        "a `..` name must never escape the target directory"
    );
}

#[test]
fn help_documents_new_subcommand() {
    // `themes --help` lists the subcommand...
    ferrocv()
        .arg("themes")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("new"));

    // ...and `themes new --help` documents what it does.
    ferrocv()
        .arg("themes")
        .arg("new")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("prelude"))
        .stdout(predicate::str::contains("--out"));
}
