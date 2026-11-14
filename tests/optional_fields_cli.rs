//! Scenario tests: every bundled theme renders every schema-valid
//! document — no theme may crash because an optional field is absent
//! or a date is less precise than `YYYY-MM-DD` (issue #291).
//!
//! JSON Resume v1.0.0 makes every field optional and accepts `YYYY`,
//! `YYYY-MM` and `YYYY-MM-DD` dates. The golden fixtures only pin a
//! full and a sparse document, so these tests sweep per-field absence:
//! starting from `render_every_field.json` (every schema field set),
//! each variant drops exactly one field (or empties one array) and
//! must render with exit 0 through every bundled theme.

use std::path::Path;

use assert_cmd::Command;
use serde_json::{Value, json};

/// Every key the JSON Resume v1.0.0 schema types as an `iso8601` date
/// (`YYYY`, `YYYY-MM` or `YYYY-MM-DD`). `certificates[].date` is the
/// exception: the schema requires a full `date` there, so
/// [`with_dates_truncated`] leaves `certificates` alone.
const DATE_KEYS: &[&str] = &["startDate", "endDate", "date", "releaseDate"];

fn every_field() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/render_every_field.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Render `doc` through `theme` in the theme's first supported format;
/// `Err` carries the exit code and stderr on failure, `Ok` the
/// rendered output file's path inside `dir`.
fn render(theme: &ferrocv::Theme, doc: &Value, dir: &Path) -> Result<std::path::PathBuf, String> {
    let format = theme.formats[0];
    let out = dir.join(format!("{}.{format}", theme.name));
    let output = Command::cargo_bin("ferrocv")
        .expect("binary `ferrocv` must be built")
        .arg("render")
        .arg("--theme")
        .arg(theme.name)
        .arg("--format")
        .arg(format.as_str())
        .arg("--output")
        .arg(&out)
        .write_stdin(serde_json::to_vec(doc).unwrap())
        .output()
        .expect("spawn ferrocv");
    if output.status.success() {
        Ok(out)
    } else {
        Err(format!(
            "exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Render every `(label, doc)` through every bundled theme, one thread
/// per theme, and fail listing every pair that did not exit 0.
fn assert_all_render(docs: &[(String, Value)]) {
    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = ferrocv::THEMES
            .iter()
            .map(|theme| {
                s.spawn(move || {
                    let tmp = tempfile::tempdir().expect("tempdir");
                    docs.iter()
                        .filter_map(|(label, doc)| {
                            render(theme, doc, tmp.path())
                                .err()
                                .map(|e| format!("{} / {label}: {e}", theme.name))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    assert!(
        failures.is_empty(),
        "{} theme/document pair(s) failed to render:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// One variant per droppable key of `obj` (at JSON pointer `ptr`), plus
/// one per array-valued key emptied to `[]`.
fn push_variants(base: &Value, ptr: &str, out: &mut Vec<(String, Value)>) {
    let obj = base.pointer(ptr).unwrap().as_object().unwrap();
    for (key, value) in obj {
        let mut doc = base.clone();
        doc.pointer_mut(ptr)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        out.push((format!("without {ptr}/{key}"), doc));
        if value.is_array() {
            let mut doc = base.clone();
            doc.pointer_mut(ptr).unwrap()[key] = json!([]);
            out.push((format!("empty {ptr}/{key}"), doc));
        }
    }
}

/// Given `render_every_field.json`, when I drop any single optional
/// field (top-level section, `basics` field, location or profile field,
/// or any field of a section entry) or empty any array, then every
/// bundled theme still renders it with exit 0.
#[test]
fn every_theme_renders_with_any_single_optional_field_omitted() {
    let base = every_field();
    let mut docs = Vec::new();
    push_variants(&base, "", &mut docs);
    push_variants(&base, "/meta", &mut docs);
    push_variants(&base, "/basics", &mut docs);
    push_variants(&base, "/basics/location", &mut docs);
    let profiles = base["basics"]["profiles"].as_array().unwrap().len();
    for i in 0..profiles {
        push_variants(&base, &format!("/basics/profiles/{i}"), &mut docs);
    }
    for (section, value) in base.as_object().unwrap() {
        if value.is_array() {
            push_variants(&base, &format!("/{section}/0"), &mut docs);
        }
    }
    assert_all_render(&docs);
}

/// Given the two minimal reproductions from issue #291 — a `YYYY` work
/// start date, and a skill with no `keywords` — when I render each
/// through every bundled theme, then each exits 0.
#[test]
fn every_theme_renders_issue_291_reproductions() {
    assert_all_render(&[
        (
            "YYYY work startDate".into(),
            json!({"basics":{"name":"A"},"work":[{"name":"X","position":"P","startDate":"2020"}]}),
        ),
        (
            "skill without keywords".into(),
            json!({"basics":{"name":"A"},"skills":[{"name":"Rust"}]}),
        ),
    ]);
}

/// `doc` with every `iso8601` date truncated to its first `len`
/// characters.
fn with_dates_truncated(doc: &Value, len: usize) -> Value {
    match doc {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let v = match v {
                        _ if k == "certificates" => v.clone(),
                        Value::String(s) if DATE_KEYS.contains(&k.as_str()) => {
                            Value::String(s[..len].to_string())
                        }
                        other => with_dates_truncated(other, len),
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| with_dates_truncated(v, len)).collect())
        }
        other => other.clone(),
    }
}

/// Given `render_every_field.json` with every date at `YYYY`,
/// `YYYY-MM`, or `YYYY-MM-DD` precision, when I render it through every
/// bundled theme, then each exits 0.
#[test]
fn every_theme_renders_every_date_precision() {
    let base = every_field();
    assert_all_render(&[
        ("YYYY dates".into(), with_dates_truncated(&base, 4)),
        ("YYYY-MM dates".into(), with_dates_truncated(&base, 7)),
        ("YYYY-MM-DD dates".into(), base),
    ]);
}

/// Given a work entry dated at each precision, when I render it through
/// `typst-jsonresume-cv`, then the date range shows the year alone for
/// `YYYY` and the month name plus year otherwise.
#[test]
fn typst_jsonresume_cv_formats_each_date_precision() {
    let theme = ferrocv::find_theme("typst-jsonresume-cv").unwrap();
    for (start, end, expected) in [
        ("1843", "1852", "1843 — 1852"),
        ("1843-01", "1852-11", "Jan 1843 — Nov 1852"),
        ("1843-01-01", "1852-11-27", "Jan 1843 — Nov 1852"),
    ] {
        let doc = json!({
            "basics": {"name": "Ada Lovelace"},
            "work": [{"name": "X", "position": "P", "startDate": start, "endDate": end}],
        });
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = render(theme, &doc, tmp.path()).unwrap_or_else(|e| panic!("{start}: {e}"));
        let text = pdf_extract::extract_text_from_mem(&std::fs::read(out).unwrap()).unwrap();
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            text.contains(expected),
            "{start}..{end}: expected {expected:?} in:\n{text}"
        );
    }
}

/// Render `doc` through the named theme and return its extracted text
/// with whitespace collapsed to single spaces.
fn render_text(theme: &str, doc: &Value) -> String {
    let theme = ferrocv::find_theme(theme).unwrap();
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = render(theme, doc, tmp.path()).unwrap_or_else(|e| panic!("{}: {e}", theme.name));
    let text = pdf_extract::extract_text_from_mem(&std::fs::read(out).unwrap()).unwrap();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Given an education entry missing `studyType` or `area`, when I
/// render it through `typst-jsonresume-cv`, then the present field
/// shows without a dangling ", ".
#[test]
fn typst_jsonresume_cv_education_omits_missing_part_without_comma() {
    for (field, value) in [("studyType", "Mathematics"), ("area", "Mathematics")] {
        let doc = json!({
            "basics": {"name": "Ada Lovelace"},
            "education": [{"institution": "University of London", field: value}],
        });
        let text = render_text("typst-jsonresume-cv", &doc);
        assert!(
            text.contains(value),
            "{field}: {value:?} missing in:\n{text}"
        );
        assert!(
            !text.contains(&format!(", {value}")) && !text.contains(&format!("{value},")),
            "{field}: dangling comma around {value:?} in:\n{text}"
        );
    }
}

/// Given a skill with no `keywords`, when I render it through
/// `fantastic-cv`, then the skill name still shows.
#[test]
fn fantastic_cv_renders_skill_name_without_keywords() {
    let doc =
        json!({"basics": {"name": "Ada Lovelace"}, "skills": [{"name": "Analytical Engines"}]});
    let text = render_text("fantastic-cv", &doc);
    assert!(
        text.contains("Analytical Engines"),
        "skill name missing in:\n{text}"
    );
}
