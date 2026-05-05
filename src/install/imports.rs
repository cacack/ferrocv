//! Scan a cached Typst Universe package's `.typ` source for transitive
//! `@preview/<name>:<version>` references.
//!
//! Used by [`super::pipeline::install_with_transitive`] to discover
//! the deps a freshly-installed package needs at compile time. The
//! scanner is deliberately lexical: a tiny quote-state machine over
//! the source string that records the contents of every double-quoted
//! string literal and tries to parse those starting with `@preview/`
//! as a [`PackageSpec`]. Anything that fails to parse is silently
//! skipped — a string that happens to look like a preview spec but
//! isn't valid is at worst a missed transitive that surfaces later as
//! a clear cache-miss at render time, not a fatal install error.
//!
//! Comments (`//` and `/* ... */`) are consumed before the string
//! scanner sees the `"`, so an `@preview/...` string inside a comment
//! does not trigger a fetch.
//!
//! Walking the package tree mirrors the conservative shape of
//! [`crate::package_cache::collect_typ_files`] (skip symlinks, recurse
//! into subdirs, only ingest case-insensitive `.typ` extensions). Per
//! CONSTITUTION §5 we do not extract a shared helper this round; the
//! third caller is the trigger to generalize.

use std::collections::HashSet;
use std::path::Path;

use super::InstallError;
use super::spec::{PackageSpec, parse_spec};

/// Scan every `.typ` file under `package_dir` for `@preview/...`
/// imports and return the deduplicated list of specs.
///
/// `self_spec` is the spec of the package being scanned; matches
/// against it (same name AND same version) are filtered out so a
/// package that legally references its own name does not trigger a
/// self-recursive install. Different versions of the same package ARE
/// kept — version coexistence is legitimate.
///
/// The result is sorted by `(name, version)` lexicographically so the
/// recursive driver's resolution order is reproducible across hosts.
pub fn scan_preview_imports(
    package_dir: &Path,
    self_spec: &PackageSpec,
) -> Result<Vec<PackageSpec>, InstallError> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut out: Vec<PackageSpec> = Vec::new();
    walk_typ_files(package_dir, self_spec, &mut seen, &mut out)?;
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
    Ok(out)
}

/// Recursively walk `dir`, scan every `.typ` file's contents for
/// `@preview/...` imports, and push deduped specs into `out`.
fn walk_typ_files(
    dir: &Path,
    self_spec: &PackageSpec,
    seen: &mut HashSet<(String, String)>,
    out: &mut Vec<PackageSpec>,
) -> Result<(), InstallError> {
    let entries = std::fs::read_dir(dir).map_err(|source| InstallError::Io {
        context: format!("read_dir {}", dir.display()),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| InstallError::Io {
            context: format!("read dir entry under {}", dir.display()),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| InstallError::Io {
            context: format!("stat {}", path.display()),
            source,
        })?;
        // Skip symlinks defensively — a corrupted cache or a local edit
        // could introduce one and following it would let the scanner
        // escape the cache root.
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            walk_typ_files(&path, self_spec, seen, out)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if !ext.eq_ignore_ascii_case("typ") {
            continue;
        }
        let src = std::fs::read_to_string(&path).map_err(|source| InstallError::Io {
            context: format!("read {}", path.display()),
            source,
        })?;
        for literal in scan_imports_in_source(&src) {
            // Only specs that parse cleanly are kept; a string that
            // starts with `@preview/` but is malformed is silently
            // skipped (see module-level docs for the rationale).
            let Ok(spec) = parse_spec(&literal) else {
                continue;
            };
            // Filter self-references: same name AND same version.
            // Different versions of the same package ARE legitimate
            // transitive deps and must not be filtered.
            if spec.name == self_spec.name && spec.version == self_spec.version {
                continue;
            }
            let key = (spec.name.clone(), spec.version.clone());
            if seen.insert(key) {
                out.push(spec);
            }
        }
    }
    Ok(())
}

/// Extract the contents of every double-quoted string literal in
/// `src` whose contents start with `@preview/`. Comments are skipped.
///
/// State machine over `char`s with two states (`Outside` and
/// `InsideString`). Line and block comments are consumed before the
/// string scanner sees them so commented-out imports are filtered.
fn scan_imports_in_source(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                // Line comment: consume to end of line.
                for cc in chars.by_ref() {
                    if cc == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                // Block comment: consume until `*/`.
                chars.next();
                let mut prev = '\0';
                for cc in chars.by_ref() {
                    if prev == '*' && cc == '/' {
                        break;
                    }
                    prev = cc;
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(cc) = chars.next() {
                    match cc {
                        '\\' => {
                            // Skip the escaped character.
                            let _ = chars.next();
                        }
                        '"' => break,
                        other => s.push(other),
                    }
                }
                if s.starts_with("@preview/") {
                    out.push(s);
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir -p");
        }
        std::fs::write(path, content).expect("write fixture");
    }

    fn dummy_self() -> PackageSpec {
        PackageSpec {
            namespace: "preview".to_owned(),
            name: "self-pkg".to_owned(),
            version: "0.1.0".to_owned(),
        }
    }

    fn make_pkg(root: &Path, files: &[(&str, &str)]) -> PathBuf {
        let pkg = root.join("pkg");
        for (rel, content) in files {
            write(&pkg.join(rel), content);
        }
        pkg
    }

    #[test]
    fn scans_single_import() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[("src/lib.typ", "#import \"@preview/foo:1.0.0\": *\n")],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "foo");
        assert_eq!(result[0].version, "1.0.0");
    }

    #[test]
    fn scans_multiple_imports_dedupes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[(
                "src/lib.typ",
                "#import \"@preview/foo:1.0.0\": *\n\
                 #import \"@preview/foo:1.0.0\": bar\n",
            )],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert_eq!(result.len(), 1, "duplicate imports must dedupe");
        assert_eq!(result[0].name, "foo");
    }

    #[test]
    fn scans_imports_across_multiple_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[
                ("src/lib.typ", "#import \"@preview/foo:1.0.0\": *\n"),
                ("src/helpers.typ", "#import \"@preview/bar:2.0.0\": *\n"),
            ],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert_eq!(result.len(), 2);
        // Result is sorted by (name, version).
        assert_eq!(result[0].name, "bar");
        assert_eq!(result[1].name, "foo");
    }

    #[test]
    fn skips_self_reference() {
        let tmp = tempfile::TempDir::new().unwrap();
        let me = dummy_self();
        let pkg = make_pkg(
            tmp.path(),
            &[(
                "src/lib.typ",
                &format!(
                    "#import \"@preview/{}:{}\": *\n\
                     #import \"@preview/{}:9.9.9\": *\n",
                    me.name, me.version, me.name,
                ),
            )],
        );
        let result = scan_preview_imports(&pkg, &me).expect("scan ok");
        // Same name+version is filtered; same name with DIFFERENT
        // version is kept (legitimate version coexistence).
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, me.name);
        assert_eq!(result[0].version, "9.9.9");
    }

    #[test]
    fn skips_line_comment_imports() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[("src/lib.typ", "// #import \"@preview/foo:1.0.0\": *\n")],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert!(
            result.is_empty(),
            "line-commented imports must be ignored: got {result:?}",
        );
    }

    #[test]
    fn skips_block_comment_imports() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[("src/lib.typ", "/* #import \"@preview/foo:1.0.0\": * */\n")],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert!(
            result.is_empty(),
            "block-commented imports must be ignored: got {result:?}",
        );
    }

    #[test]
    fn skips_invalid_specs_silently() {
        let tmp = tempfile::TempDir::new().unwrap();
        // `@preview/has spaces:1.0.0` is rejected by parse_spec because
        // whitespace is illegal in a name. The scanner must skip it
        // without surfacing an error.
        let pkg = make_pkg(
            tmp.path(),
            &[("src/lib.typ", "#import \"@preview/has spaces:1.0.0\"\n")],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert!(result.is_empty(), "invalid specs must be skipped silently");
    }

    #[test]
    fn non_typ_files_are_ignored() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[
                ("src/lib.typ", "#import \"@preview/foo:1.0.0\": *\n"),
                (
                    "README.md",
                    "see also @preview/bar:2.0.0 — but markdown is ignored\n\
                     ```\n#import \"@preview/baz:3.0.0\"\n```\n",
                ),
            ],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        assert_eq!(result.len(), 1, "only .typ files are scanned");
        assert_eq!(result[0].name, "foo");
    }

    #[test]
    fn result_order_is_deterministic() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg = make_pkg(
            tmp.path(),
            &[(
                "src/lib.typ",
                "#import \"@preview/zeta:1.0\": *\n\
                 #import \"@preview/alpha:1.0\": *\n\
                 #import \"@preview/mu:1.0\": *\n",
            )],
        );
        let result = scan_preview_imports(&pkg, &dummy_self()).expect("scan ok");
        let names: Vec<&str> = result.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }
}
