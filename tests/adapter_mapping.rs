//! Scenario tests: the vendored adapters map JSON Resume fields to the
//! right place — every PDF link targets its source URL, and fields the
//! upstream templates dropped or mapped to the wrong slot render (issue #293).

use std::path::Path;

use assert_cmd::Command;
use pdf_extract::{Document, Object};
use serde_json::{Value, json};

/// The vendored adapters (CONSTITUTION §4); native themes author their
/// links directly against the schema.
const ADAPTERS: &[&str] = &[
    "typst-jsonresume-cv",
    "fantastic-cv",
    "modern-cv",
    "basic-resume",
];

fn full_fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/render_full.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Render `doc` through `theme` to PDF via the CLI and return the bytes.
fn render_pdf(theme: &str, doc: &Value) -> Vec<u8> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("resume.pdf");
    Command::cargo_bin("ferrocv")
        .expect("binary `ferrocv` must be built")
        .args(["render", "--theme", theme, "--format", "pdf", "--output"])
        .arg(&out)
        .write_stdin(serde_json::to_vec(doc).unwrap())
        .assert()
        .success();
    std::fs::read(out).unwrap()
}

/// Render `doc` through `theme` and return its extracted text with
/// whitespace collapsed to single spaces.
fn render_text(theme: &str, doc: &Value) -> String {
    let text = pdf_extract::extract_text_from_mem(&render_pdf(theme, doc)).unwrap();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every `/URI` link-annotation target in `pdf`, in page order.
fn link_targets(pdf: &[u8]) -> Vec<String> {
    let doc = Document::load_mem(pdf).expect("valid PDF");
    let mut targets = Vec::new();
    for page in doc.get_pages().into_values() {
        for annot in doc.get_page_annotations(page).unwrap_or_default() {
            let Ok(action) = annot.get_deref(b"A", &doc).and_then(Object::as_dict) else {
                continue;
            };
            if let Ok(uri) = action.get_deref(b"URI", &doc).and_then(Object::as_str) {
                targets.push(String::from_utf8_lossy(uri).into_owned());
            }
        }
    }
    targets
}

/// Every string value under a `url` key anywhere in `v`.
fn source_urls(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (key, value) in map {
                match (key.as_str(), value) {
                    ("url", Value::String(url)) => out.push(url.clone()),
                    _ => source_urls(value, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| source_urls(item, out)),
        _ => {}
    }
}

/// Given `doc`, when I render it through every adapter, then every link
/// annotation is exactly a source URL, `mailto:<email>`, or
/// `tel:<phone>`, and `basics.url`, the email, every profile URL, and
/// (where the adapter has the section) the project and certificate URLs
/// are among them.
fn assert_links_are_source_urls(doc: &Value) {
    let mut allowed = Vec::new();
    source_urls(doc, &mut allowed);
    let basics = &doc["basics"];
    let site = basics["url"].as_str().unwrap().to_owned();
    let mailto = format!("mailto:{}", basics["email"].as_str().unwrap());
    allowed.push(mailto.clone());
    allowed.push(format!("tel:{}", basics["phone"].as_str().unwrap()));
    let profile_urls: Vec<String> = basics["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["url"].as_str().unwrap().to_owned())
        .collect();
    // Upstream typst-jsonresume-cv links no project or certificate URL,
    // and modern-cv links no certificate URL.
    let project = doc["projects"][0]["url"].as_str().unwrap().to_owned();
    let certificate = doc["certificates"][0]["url"].as_str().unwrap().to_owned();

    for theme in ADAPTERS {
        let targets = link_targets(&render_pdf(theme, doc));
        for target in &targets {
            assert!(
                allowed.contains(target),
                "{theme}: link target {target:?} is not a source URL; allowed: {allowed:?}"
            );
        }
        let mut expected = vec![site.clone(), mailto.clone()];
        expected.extend(profile_urls.iter().cloned());
        if !matches!(*theme, "typst-jsonresume-cv") {
            expected.push(project.clone());
        }
        if matches!(*theme, "fantastic-cv" | "basic-resume") {
            expected.push(certificate.clone());
        }
        for expected in &expected {
            assert!(
                targets.contains(expected),
                "{theme}: no link to {expected:?}; got {targets:?}"
            );
        }
    }
}

#[test]
fn every_adapter_links_exactly_the_source_urls() {
    assert_links_are_source_urls(&full_fixture());
}

/// An `http://` URL must not be rewritten to `https://`, and a full
/// URL must not gain a second scheme.
#[test]
fn every_adapter_links_http_urls_unchanged() {
    let mut doc = full_fixture();
    doc["basics"]["url"] = json!("http://example.com/ada");
    doc["basics"]["profiles"][0]["url"] = json!("http://github.example.com/ada");
    doc["projects"][0]["url"] = json!("http://example.com/notes-g");
    doc["certificates"][0]["url"] = json!("http://example.com/cert");
    assert_links_are_source_urls(&doc);
}

/// Given `basics.url` and GitHub/LinkedIn profiles, when I render
/// through `modern-cv`, then the contact line shows all three.
#[test]
fn modern_cv_renders_url_and_profiles() {
    let text = render_text("modern-cv", &full_fixture());
    for expected in [
        "https://example.com/ada",
        "GitHub: ada",
        "LinkedIn: ada-lovelace",
    ] {
        assert!(text.contains(expected), "{expected:?} missing in:\n{text}");
    }
}

/// Given an education entry with a `score`, when I render through
/// `basic-resume`, then the score shows.
#[test]
fn basic_resume_renders_education_score() {
    let mut doc = full_fixture();
    doc["education"][0]["score"] = json!("3.9/4.0");
    let text = render_text("basic-resume", &doc);
    assert!(text.contains("GPA: 3.9/4.0"), "score missing in:\n{text}");
}

/// Given an ongoing role and an education entry with a `score` and
/// `courses`, when I render through `fantastic-cv`, then the role
/// reads "Present" and the score and courses show.
#[test]
fn fantastic_cv_renders_present_score_and_courses() {
    let mut doc = full_fixture();
    doc["work"][0].as_object_mut().unwrap().remove("endDate");
    doc["education"][0]["score"] = json!("3.9/4.0");
    doc["education"][0]["courses"] = json!(["Calculus"]);
    let text = render_text("fantastic-cv", &doc);
    for expected in [
        "1843-01-01 — Present",
        "GPA: 3.9/4.0",
        "Selected coursework : Calculus",
    ] {
        assert!(text.contains(expected), "{expected:?} missing in:\n{text}");
    }
}

/// The sparse golden pins `fantastic-cv`'s degradation for absent data:
/// no separator may dangle at the end of a line.
#[test]
fn fantastic_cv_sparse_golden_has_no_dangling_separators() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/fantastic-cv-sparse.txt");
    let golden = std::fs::read_to_string(path).unwrap();
    for line in golden.lines() {
        let line = line.trim_end();
        assert!(
            !(line.ends_with('—') || line.ends_with('|') || line.ends_with(':')),
            "dangling separator: {line:?}"
        );
    }
    assert!(golden.contains("Present"), "ongoing role lacks Present");
}
