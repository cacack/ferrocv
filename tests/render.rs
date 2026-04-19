//! Integration tests for the `ferrocv::compile_pdf` library API.
//!
//! These tests exercise the **real** embedded Typst compiler — no
//! mocks, no stubs, no feature-gated "lite" mode (CONSTITUTION.md
//! Testing doctrine §4). They also enforce the no-network commitment
//! (CONSTITUTION.md §6.1) by asserting that `@preview/...` package
//! imports are rejected at the World layer rather than fetched from
//! the Typst Universe registry.
//!
//! Coverage:
//! 1. Smoke — minimal Typst source compiles and produces PDF bytes.
//! 2. JSON injection round-trip — `json("/resume.json")` reads the
//!    `serde_json::Value` passed to `compile_pdf`, and an injected
//!    sentinel string survives into the PDF byte stream.
//! 3. Compile error surfaces a useful diagnostic.
//! 4. Package fetch is rejected — proves no network resolver is wired
//!    up. This test is the in-code enforcement of CONSTITUTION §6.1.

use ferrocv::{RenderError, compile_pdf};
use serde_json::{Value, json};

/// PDF magic bytes; every valid PDF stream must start with `%PDF-`.
const PDF_MAGIC: &[u8] = b"%PDF-";

#[test]
fn smoke_minimal_source_compiles_to_pdf() {
    let bytes = compile_pdf("= Hello", &Value::Object(Default::default()))
        .expect("minimal Typst source must compile");
    assert!(
        bytes.starts_with(PDF_MAGIC),
        "output must begin with PDF magic; got first 16 bytes: {:?}",
        &bytes[..bytes.len().min(16)],
    );
    assert!(
        bytes.len() > 100,
        "PDF output suspiciously small ({} bytes)",
        bytes.len(),
    );
}

#[test]
fn json_injection_round_trips_sentinel_into_pdf() {
    // Sentinel is short ASCII (letters + digits) so it survives PDF
    // text encoding without splitting across operators. We then search
    // the raw bytes — no PDF text-extraction crate needed. If a future
    // PDF version compresses by default and breaks this, swap to the
    // `pdf-extract` dev-dep approach documented in the prompt.
    let sentinel = "RenderTestSentinel9c4a1e";
    let data = json!({ "basics": { "name": sentinel } });

    // Note the leading slash on `/resume.json` — Typst's `json()` is
    // path-based and the World resolves this `FileId` to the bytes of
    // `data` serialized via `serde_json::to_vec`.
    let source = r#"
#let r = json("/resume.json")
= #r.basics.name
"#;

    let bytes = compile_pdf(source, &data).expect("source must compile");
    assert!(bytes.starts_with(PDF_MAGIC));

    let needle = sentinel.as_bytes();
    let found = bytes.windows(needle.len()).any(|w| w == needle);
    assert!(
        found,
        "sentinel `{sentinel}` not found in PDF byte stream ({} bytes)",
        bytes.len(),
    );
}

#[test]
fn invalid_source_returns_render_error_with_message() {
    // `#let x =` is an incomplete expression — Typst rejects at parse
    // or compile time, depending on version. Either way, we should
    // get an `Err(RenderError)` with a non-empty Display.
    let err: RenderError = compile_pdf("#let x =", &Value::Object(Default::default()))
        .expect_err("incomplete `#let` expression must fail to compile");
    let rendered = format!("{err}");
    assert!(
        !rendered.trim().is_empty(),
        "RenderError Display must produce a non-empty diagnostic",
    );
    // The Display impl prefixes each diagnostic with `error:`; assert
    // on that contract so format regressions are caught.
    assert!(
        rendered.contains("error:"),
        "RenderError Display must format diagnostics as `error: ...`; got:\n{rendered}",
    );
}

#[test]
fn preview_package_import_is_rejected_no_network() {
    // CONSTITUTION §6.1 enforcement: any `@preview/...` import is a
    // request for the Typst Universe package registry, which would
    // require a network fetch. Our World has no package resolver, so
    // this MUST surface as a compile error rather than silently
    // succeeding (or, worse, attempting a network call).
    //
    // We don't pin to an exact diagnostic phrasing because Typst's
    // PackageError variants format differently across versions; we
    // just assert the package coordinates leak through somewhere in
    // the diagnostic text so a user can see what was rejected.
    let source = r#"#import "@preview/cetz:0.2.0": *"#;
    let err = compile_pdf(source, &Value::Object(Default::default()))
        .expect_err("preview-package import must be rejected (no network resolver)");
    let rendered = format!("{err}").to_lowercase();
    assert!(
        rendered.contains("preview")
            || rendered.contains("cetz")
            || rendered.contains("package")
            || rendered.contains("not found"),
        "diagnostic must mention the rejected package or 'not found'; got:\n{rendered}",
    );
}
