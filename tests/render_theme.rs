//! Golden-file tests for the PDF-target themes — the vendored adapters
//! (`typst-jsonresume-cv`, `fantastic-cv`, `modern-cv`, `basic-resume`)
//! and the native `classic` theme. The shared `run_golden` helper drives
//! them all; text and HTML themes have their own harnesses in
//! `render_text.rs` and `render_html.rs`.
//!
//! Enforces CONSTITUTION testing doctrine:
//! - **§2** — every theme has a golden-file test. We compile the
//!   theme against a committed fixture, extract the rendered text
//!   with `pdf-extract`, and compare against a committed reference.
//!   PDF bytes are fragile across Typst patch versions; a normalized
//!   text extraction is substantially more stable and still catches
//!   real regressions (missing sections, re-ordered fields, lost
//!   fields, etc.).
//! - **§4** — no mocking Typst. These tests drive the real embedded
//!   compiler via [`ferrocv::compile_theme`] against the real vendored
//!   theme sources. No stubs, no feature-gated "lite" mode.
//!
//! Also indirectly exercises **§6.1** (no network at render time): the
//! adapter imports nothing under `@preview/...`, and `compile_theme`
//! goes through the same [`FerrocvWorld`] that rejects package
//! coordinates unconditionally. If a future vendored-theme update
//! introduced a network-requiring import, these tests would fail
//! loudly (the compile would error) rather than silently succeeding.
//!
//! # Fixture coverage
//!
//! Two fixtures are exercised, each pinned to its own golden so that a
//! regression in either "shape" shows up as a committed diff:
//!
//! - `render_full.json` (Ada Lovelace) — every field the theme reads
//!   is populated, including all optional contact fields, all section
//!   arrays, and `meta.language`. Pins the "full-field" output shape.
//! - `render_sparse.json` (Grace Hopper) — a schema-valid but
//!   deliberately-sparse document: no `meta`, no `basics.location`,
//!   no `basics.email`/`phone`, no `projects`/`education`/`skills`.
//!   Pins the optional-field shim in
//!   `assets/themes/typst-jsonresume-cv/resume.typ`
//!   (see that file's `VENDORING.md` patch #3). A silent regression
//!   in how missing fields degrade becomes a committed diff here.
//!
//! Fixture contents use public-domain pioneers of computing to keep
//! test data PII-free and recognizable at a glance. These fixtures are
//! *test-shaped* — narrow, deliberate coverage of specific adapter
//! code paths. A realistic, demo-quality resume (the kind a user
//! would actually author) belongs in the forkable starter-template
//! repo tracked by issue #43, not here.
//!
//! # Updating the goldens
//!
//! If a Typst patch bump or an intentional theme edit legitimately
//! shifts the rendered text, regenerate with:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --test render_theme
//! ```
//!
//! Inspect the diffs under `tests/goldens/` before committing. A
//! golden update should always have an obvious cause; an unexplained
//! diff is a regression, not a golden bump.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// PDF magic bytes; every valid PDF stream must start with `%PDF-`.
const PDF_MAGIC: &[u8] = b"%PDF-";

/// Environment variable that, when set to any non-empty value, rewrites
/// the relevant golden with the current run's normalized output
/// instead of asserting equality.
const UPDATE_ENV: &str = "UPDATE_GOLDEN";

// typst-jsonresume-cv adapter goldens.
#[test]
fn typst_jsonresume_cv_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "typst-jsonresume-cv",
        "tests/fixtures/render_full.json",
        "tests/goldens/typst-jsonresume-cv.txt",
        "Ada Lovelace",
    );
}

#[test]
fn typst_jsonresume_cv_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "typst-jsonresume-cv",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/typst-jsonresume-cv-sparse.txt",
        "Grace Hopper",
    );
}

// fantastic-cv adapter goldens.
#[test]
fn fantastic_cv_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "fantastic-cv",
        "tests/fixtures/render_full.json",
        "tests/goldens/fantastic-cv.txt",
        "Ada Lovelace",
    );
}

#[test]
fn fantastic_cv_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "fantastic-cv",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/fantastic-cv-sparse.txt",
        "Grace Hopper",
    );
}

// modern-cv adapter goldens.
#[test]
fn modern_cv_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "modern-cv",
        "tests/fixtures/render_full.json",
        "tests/goldens/modern-cv.txt",
        "Ada Lovelace",
    );
}

#[test]
fn modern_cv_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "modern-cv",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/modern-cv-sparse.txt",
        "Grace Hopper",
    );
}

// basic-resume adapter goldens.
#[test]
fn basic_resume_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "basic-resume",
        "tests/fixtures/render_full.json",
        "tests/goldens/basic-resume.txt",
        "Ada Lovelace",
    );
}

#[test]
fn basic_resume_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "basic-resume",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/basic-resume-sparse.txt",
        "Grace Hopper",
    );
}

// classic native-theme goldens (PDF-first; same PDF→text→normalize
// pipeline as the adapters above, since `classic` is designed for PDF).
#[test]
fn classic_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "classic",
        "tests/fixtures/render_full.json",
        "tests/goldens/classic.txt",
        "Ada Lovelace",
    );
}

#[test]
fn classic_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "classic",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/classic-sparse.txt",
        "Grace Hopper",
    );
}

/// Exercises the section renderers the two shared fixtures omit —
/// volunteer, awards, publications, languages, references. `render_full`
/// and `render_sparse` cover basics/work/education/projects/skills/
/// interests/certificates; without this, those five renderers would have
/// no golden coverage and could regress silently (testing-doctrine §2).
/// This fixture is `classic`-scoped on purpose: extending the shared
/// `render_full.json` would force regenerating every other theme's golden.
#[test]
fn classic_renders_all_sections_to_expected_text() {
    run_golden(
        "classic",
        "tests/fixtures/render_sections.json",
        "tests/goldens/classic-sections.txt",
        "Alan Turing",
    );
}

/// An entry taller than one page must flow onto the next page instead
/// of being clipped (#290). `render_long_entry.json` is a single `work`
/// entry with 60 numbered highlights of ~140 characters each — roughly
/// two and a half pages. Every one must be visible on some page.
///
/// Plain `pdf-extract` text is not enough here: clipped overflow is
/// still in the page's content stream, just drawn below the page edge,
/// so it extracts fine. [`visible_text`] keeps only glyphs inside the
/// page's media box — what a reader (or `pdftotext`) actually sees.
#[test]
fn classic_long_entry_keeps_every_highlight() {
    let data: Value = serde_json::from_slice(
        &fs::read(crate_path("tests/fixtures/render_long_entry.json"))
            .expect("read long-entry fixture"),
    )
    .expect("parse long-entry fixture");
    let theme = ferrocv::find_theme("classic").expect("classic must be registered");
    let bytes = ferrocv::compile_theme(theme, &data).expect("classic must compile long entry");
    let text = visible_text(&bytes);

    let missing: Vec<usize> = (1..=60)
        .filter(|n| !text.contains(&format!("Highlight{n:02}:")))
        .collect();
    assert!(
        missing.is_empty(),
        "classic clipped highlights {missing:?} of a page-spanning entry"
    );
}

/// Extract the text of `pdf` whose glyph origin lies inside its page's
/// media box, with all whitespace removed (word-spacing reconstruction
/// is irrelevant to the marker search and would only add noise).
fn visible_text(pdf: &[u8]) -> String {
    use pdf_extract::{MediaBox, OutputDev, OutputError, Transform};

    #[derive(Default)]
    struct Visible {
        lly: f64,
        ury: f64,
        out: String,
    }
    impl OutputDev for Visible {
        fn begin_page(
            &mut self,
            _: u32,
            media_box: &MediaBox,
            _: Option<(f64, f64, f64, f64)>,
        ) -> Result<(), OutputError> {
            self.lly = media_box.lly;
            self.ury = media_box.ury;
            Ok(())
        }
        fn end_page(&mut self) -> Result<(), OutputError> {
            Ok(())
        }
        fn output_character(
            &mut self,
            trm: &Transform,
            _: f64,
            _: f64,
            _: f64,
            ch: &str,
        ) -> Result<(), OutputError> {
            if (self.lly..=self.ury).contains(&trm.m32) {
                self.out.extend(ch.chars().filter(|c| !c.is_whitespace()));
            }
            Ok(())
        }
        fn begin_word(&mut self) -> Result<(), OutputError> {
            Ok(())
        }
        fn end_word(&mut self) -> Result<(), OutputError> {
            Ok(())
        }
        fn end_line(&mut self) -> Result<(), OutputError> {
            Ok(())
        }
    }

    let doc = pdf_extract::Document::load_mem(pdf).expect("lopdf must parse the compiled PDF");
    let mut dev = Visible::default();
    pdf_extract::output_doc(&doc, &mut dev).expect("pdf-extract must walk the compiled PDF");
    dev.out
}

/// Page breaks never strand a heading (#290): neither a section heading
/// nor an entry's title line may be the last line on a page with its
/// body pushed onto the next. A fixed fixture only probes one break
/// position, so this sweeps one: `k` one-line filler highlights in the
/// first role shift every later line down by one per step, walking each
/// entry title and the `EDUCATION` heading across the page boundary.
/// Every highlight is short enough to stay on one line, so the last line
/// of every non-final page must be a `•` bullet.
///
/// The ranges are calibrated to today's page geometry: 11 three-bullet
/// roles fill page one, and 25 steps of `k` exceed one role's height
/// (title plus three bullets plus spacing), so the boundary visits every
/// line of an entry and the Experience-to-Education transition. Because
/// a geometry change could quietly stop the sweep from reaching those
/// boundaries, the test also requires that some step opens a page with
/// `EDUCATION` and some step opens one with an entry title.
#[test]
fn classic_page_breaks_never_strand_a_heading() {
    let theme = ferrocv::find_theme("classic").expect("classic must be registered");
    let first_line = |page: &str| {
        page.lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_owned()
    };
    let (mut heading_crossed, mut title_crossed) = (false, false);
    for k in 0..=24 {
        let mut work = vec![serde_json::json!({
            "name": "Filler Co",
            "position": "Engineer",
            "startDate": "1900-01-01",
            "highlights": (1..=k).map(|j| format!("Filler {j}.")).collect::<Vec<_>>(),
        })];
        for i in 1..=11 {
            work.push(serde_json::json!({
                "name": format!("Employer {i}"),
                "position": format!("Engineer {i}"),
                "startDate": format!("{}-01-01", 1900 + i),
                "highlights": (1..=3).map(|j| format!("Role {i} bullet {j}.")).collect::<Vec<_>>(),
            }));
        }
        let data = serde_json::json!({
            "basics": { "name": "Grace Hopper" },
            "work": work,
            "education": [ {
                "institution": "Yale University",
                "studyType": "PhD",
                "startDate": "1930-01-01",
            } ],
        });
        let bytes = ferrocv::compile_theme(theme, &data)
            .unwrap_or_else(|e| panic!("classic must compile sweep k={k}: {e}"));
        let pages = pdf_extract::extract_text_from_mem_by_pages(&bytes)
            .expect("pdf-extract must parse the compiled PDF");
        assert!(pages.len() > 1, "sweep k={k} must span pages");
        for page in &pages[1..] {
            let first = first_line(page);
            heading_crossed |= first.starts_with("EDUCATION");
            title_crossed |= first.starts_with("Engineer");
        }
        for (n, page) in pages[..pages.len() - 1].iter().enumerate() {
            let last = page.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("");
            assert!(
                last.trim_start().starts_with('•'),
                "sweep k={k}: page {} ends with a non-body line {last:?}; \
                 a heading or entry title was stranded:\n{page}",
                n + 1,
            );
        }
    }
    assert!(
        heading_crossed && title_crossed,
        "the sweep no longer walks a heading (crossed: {heading_crossed}) and an \
         entry title (crossed: {title_crossed}) across the page boundary; \
         recalibrate its ranges to the current page geometry",
    );
}

/// Audience-aware rendering (issue #182, exercising the `ext` accessor
/// from #180): `classic` surfaces a `meta.x-audience` string as a
/// "Tailored for: <label>" tagline. This is intentionally NOT a committed
/// golden — it asserts the tagline appears (and its absence when no tag is
/// present) by extracting the compiled PDF text, mirroring `run_golden`'s
/// PDF→text pipeline but with inline data so no fixture/golden is needed.
///
/// The tag lives under `meta` (not the document root) because the JSON
/// Resume schema forbids unknown root properties but permits `x-`
/// extensions inside objects — so a real `ferrocv render` of such a
/// document validates and renders the tagline.
#[test]
fn classic_renders_audience_tag_from_meta() {
    let theme = ferrocv::find_theme("classic").expect("classic must be registered");

    // With `meta.x-audience`, the tagline appears.
    let tagged = serde_json::json!({
        "meta": { "x-audience": "security" },
        "basics": { "name": "Ada Lovelace", "label": "Engineer" },
        "work": [ { "position": "Engineer", "name": "Acme", "startDate": "2020" } ],
    });
    let bytes = ferrocv::compile_theme(theme, &tagged)
        .expect("classic must compile the audience-tagged document");
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .expect("pdf-extract must parse the compiled PDF");
    assert!(
        text.contains("Tailored for: security"),
        "a meta.x-audience tag must render as a 'Tailored for:' tagline; got:\n{text}"
    );
    // Exactly one tagline — guards against a renderer that duplicated it
    // or emitted it in addition to some other (mislocated) copy.
    assert_eq!(
        text.matches("Tailored for:").count(),
        1,
        "the audience tagline must render exactly once; got:\n{text}"
    );

    // Without the tag, no tagline (and no stray 'Tailored for:' text).
    let untagged = serde_json::json!({
        "basics": { "name": "Ada Lovelace", "label": "Engineer" },
        "work": [ { "position": "Engineer", "name": "Acme", "startDate": "2020" } ],
    });
    let bytes = ferrocv::compile_theme(theme, &untagged)
        .expect("classic must compile the untagged document");
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .expect("pdf-extract must parse the compiled PDF");
    assert!(
        !text.contains("Tailored for:"),
        "no tagline must render when meta.x-audience is absent; got:\n{text}"
    );
}

/// Shared golden-file workflow: compile the named adapter against the
/// named fixture, extract and normalize the PDF text, and compare
/// against (or rewrite, under `UPDATE_GOLDEN`) the committed golden.
///
/// `required_name` is a substring the extracted text must contain
/// before we'll consider writing a golden from it — a cheap sanity
/// check that `pdf-extract` hasn't degenerated and we're not about to
/// freeze garbage.
fn run_golden(theme_name: &str, fixture: &str, golden: &str, required_name: &str) {
    let fixture_path = crate_path(fixture);
    let fixture_bytes = fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", fixture_path.display()));
    let data: Value = serde_json::from_slice(&fixture_bytes)
        .unwrap_or_else(|e| panic!("parse fixture {}: {e}", fixture_path.display()));

    let theme = ferrocv::find_theme(theme_name)
        .unwrap_or_else(|| panic!("theme `{theme_name}` must be registered in THEMES"));

    let bytes = ferrocv::compile_theme(theme, &data).unwrap_or_else(|e| {
        panic!(
            "{theme_name} must compile against {}: {e}",
            fixture_path.display()
        )
    });

    assert!(
        bytes.starts_with(PDF_MAGIC),
        "compiled output must begin with PDF magic; got first 16 bytes: {:?}",
        &bytes[..bytes.len().min(16)],
    );
    assert!(
        bytes.len() > 1024,
        "PDF output suspiciously small ({} bytes) — expected a real multi-page resume",
        bytes.len(),
    );

    let raw = pdf_extract::extract_text_from_mem(&bytes)
        .expect("pdf-extract must parse compiled PDF bytes");
    let normalized = normalize(&raw);

    // Sanity check runs BEFORE any golden write. If the extractor ever
    // returns empty or degenerate text, we refuse to regenerate the
    // golden from that garbage.
    assert!(
        normalized.contains(required_name),
        "extracted PDF text must contain the fixture's name `{required_name}`; \
         got {} bytes of normalized text starting with:\n{}",
        normalized.len(),
        normalized.chars().take(200).collect::<String>(),
    );

    let golden_path = crate_path(golden);

    if std::env::var_os(UPDATE_ENV).is_some_and(|v| !v.is_empty()) {
        if let Some(parent) = golden_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("create {}: {e}", parent.display()));
        }
        fs::write(&golden_path, &normalized)
            .unwrap_or_else(|e| panic!("write golden {}: {e}", golden_path.display()));
        eprintln!(
            "{UPDATE_ENV} set: wrote {} bytes to {}. \
             Re-run without {UPDATE_ENV} to verify.",
            normalized.len(),
            golden_path.display(),
        );
        return;
    }

    let expected_raw = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!(
            "read golden {}: {e}\nTo create it, run: \
             UPDATE_GOLDEN=1 cargo test --test render_theme",
            golden_path.display(),
        )
    });
    // Normalize the on-disk golden too so CRLF-delimited checkouts
    // (Windows + git autocrlf) match the always-LF output of
    // `normalize()`. `.gitattributes` pins the file to LF for new
    // checkouts, but pre-existing clones may still have CRLF.
    let expected = normalize(&expected_raw);

    assert_eq!(
        normalized,
        expected,
        "theme golden mismatch for {}. \
         Regenerate with `UPDATE_GOLDEN=1 cargo test --test render_theme` \
         if the difference is expected (e.g., Typst version bump).",
        golden_path.display(),
    );
}

/// Normalize extracted PDF text so the golden is stable against
/// trivial whitespace differences without hiding real regressions.
///
/// Kept deliberately narrow:
/// - trim trailing whitespace on each line
/// - collapse runs of blank lines to a single blank
/// - strip leading and trailing blank lines
/// - ensure exactly one trailing newline
///
/// Do NOT lowercase, Unicode-normalize, or collapse intra-line
/// whitespace — those transformations would hide rendering
/// regressions the golden is meant to catch.
fn normalize(raw: &str) -> String {
    let mut lines: Vec<String> = raw.lines().map(|l| l.trim_end().to_string()).collect();
    let mut collapsed: Vec<String> = Vec::with_capacity(lines.len());
    let mut prev_blank = false;
    for line in lines.drain(..) {
        let is_blank = line.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        collapsed.push(line);
        prev_blank = is_blank;
    }
    while collapsed.first().is_some_and(|s| s.is_empty()) {
        collapsed.remove(0);
    }
    while collapsed.last().is_some_and(|s| s.is_empty()) {
        collapsed.pop();
    }
    let mut out = collapsed.join("\n");
    out.push('\n');
    out
}

/// Resolve a path relative to the crate root (where `Cargo.toml`
/// lives). Cargo sets `CARGO_MANIFEST_DIR` for integration tests, so
/// this works regardless of the shell's `cwd` at test time.
fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}
