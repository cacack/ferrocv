# Typst HTML export viability — issue #44

- **Research branch**: `research/44-html-viability` @ `dbc35bd`
- **Date**: 2026-04-19
- **Versions investigated**: `typst 0.13.1` (current pin) and `typst 0.14.2`
  (latest stable); `typst-html 0.13.1` and `typst-html 0.14.2`

## TL;DR

**YELLOW — proceed with caveats.** Typst HTML export is functional enough
to ship for two of the three registered themes (`text-minimal` and
`typst-jsonresume-cv`), is single-file out of the box with zero external
assets, and upholds every CONSTITUTION §6 trust commitment. The caveats
are not small, though:

1. **Typst 0.14 bump is a prerequisite**, not optional — on `0.13.1`
   every theme dies at `#set page(...)` with a hard error, fixed only in
   `0.14.1` which downgrades page rules to warnings in HTML mode.
2. **Typst itself still labels HTML export "experimental"** and tells
   users "Do not use this feature for production use cases"; we must
   require `Feature::Html` be opted in on the `Library` and accept API
   breakage risk between minors. (Our `--format html` CLI surface can
   still be stable; what's experimental is upstream's HTML internals.)
3. **`fantastic-cv` does not compile to HTML** as-is (HTML rejects
   `link("")` where PDF tolerated it). Shipping HTML for that adapter
   needs a glue-layer patch, deferrable to a follow-up.

The single most decision-shaping finding: the `0.13.1` compiler does
*not* produce HTML for any of our themes, so the 0.14 bump stops being
deferrable the moment we commit to shipping `--format html`.

## typst-html crate — API and version requirements

**Entry function** (identical signature in 0.13.1 and 0.14.2):

```rust
pub fn html(document: &HtmlDocument) -> SourceResult<String>
```

Source: [docs.rs/typst-html/0.14.2/typst_html/fn.html.html](https://docs.rs/typst-html/0.14.2/typst_html/fn.html.html),
[docs.rs/typst-html/0.13.1/typst_html/fn.html.html](https://docs.rs/typst-html/0.13.1/typst_html/fn.html.html).

- **Input**: `HtmlDocument` (re-exported from `typst_html::HtmlDocument`
  in 0.14; lives at `typst_library::html::HtmlDocument` internally).
- **Output**: a `String` (the full HTML document, including `<!DOCTYPE
  html>`). Not bytes.
- **Target selection for `typst::compile`**: use the existing generic
  dispatch. Both 0.13 and 0.14 expose `typst::compile<D: Document>`; we
  already call `typst::compile::<PagedDocument>(&world)` for PDF. For
  HTML the only change is the type parameter:

  ```rust
  let Warned { output, .. } = typst::compile::<HtmlDocument>(&world);
  ```

- **Library-level gate**: HTML compilation **requires** the
  `Feature::Html` flag on the `Library`. Calling
  `typst::compile::<HtmlDocument>` against `Library::default()` returns
  a diagnostic "html export is only available when `--features html` is
  passed". Enabling is a one-liner:

  ```rust
  let features: Features = [Feature::Html].into_iter().collect();
  let lib = Library::builder().with_features(features).build();
  ```

  (On 0.13 the builder came from `LibraryBuilder::default()`; on 0.14
  `LibraryBuilder` is constructed via `Library::builder()` — the
  `LibraryExt` trait needs to be in scope.)
  Source: `warn_or_error_for_html` in
  [typst-library/src/lib.rs](https://github.com/typst/typst/blob/v0.14.2/crates/typst-library/src/lib.rs).

- **typst-html 0.13.1 exists** and depends on `typst-library ^0.13.1`.
  It is API-compatible with the current pin *at the crate level* but —
  as the POC below demonstrates — the 0.13 HTML backend treats
  `#set page` as a fatal error, which kills every theme before a byte
  of HTML is emitted. See §3.

- **typst-html 0.14.2** depends on `typst-library ^0.14.2` (and
  siblings: `typst-svg`, `typst-syntax`, `typst-utils`, `typst-timing`,
  `typst-macros`, `typst-assets`, all `^0.14.2`). Not usable alongside
  a `typst ^0.13` pin.

## Typst HTML export maturity — upstream state

- **Typst 0.14.2 release notes** make no change to HTML status;
  HTML-mode work continues.
  [typst 0.14.2 release](https://github.com/typst/typst/releases/tag/v0.14.2).
- **Typst 0.14.1** is the first version where `#set page` in HTML mode
  became a warning rather than a hard error. Quote from the release
  notes: *"A `page` set rule in HTML export is now a warning instead of
  a hard error, in line with how unsupported elements are generally
  treated."*
  [typst 0.14.1 release](https://github.com/typst/typst/releases/tag/v0.14.1).
- **Typst 0.14.0** broadly expanded HTML support: "typed HTML API",
  full Model-category element coverage, intra-doc link targets,
  outline/bibliography/footnote support, syntax highlighting in code
  blocks. The release notes do *not* claim stabilization.
  [typst 0.14.0 release](https://github.com/typst/typst/releases/tag/v0.14.0).
- **Typst 0.13.0** (first HTML release): *"HTML export is currently
  under active development. The feature is still very incomplete, but
  already available for experimentation behind a feature flag."*
  [typst 0.13.0 release](https://github.com/typst/typst/releases/tag/v0.13.0).
- **Current Typst reference docs (`typst.app/docs/reference/html/`)
  banner, verbatim**:
  > "Typst's HTML export is currently under active development. The
  > feature is still very incomplete and only available for
  > experimentation behind a feature flag. Do not use this feature for
  > production use cases."
- **Upstream tracking issue [#5512](https://github.com/typst/typst/issues/5512)**:
  HTML export remains "available for experimentation, but not intended
  for production use." Gap list: math elements unimplemented, layout
  elements (box/block) partially working, visualize elements (shapes,
  curves) non-functional, CSS and EPUB deferred, accessibility audit
  pending.
- **Typst library code still gates HTML behind `Feature::Html`** with
  the comment "Enables in-development features that may be changed or
  removed at any time" — so experimental status is still codified in
  the library as of 0.14.2.

**What this means for us**: the *CLI surface* we expose to users
(`--format html`) can be stable. The *internals* we're wrapping are not
— we should expect the HTML emitter's output shape to shift between
Typst minors, which makes HTML golden-file tests fragile and argues for
loose assertions (presence of key tags + extracted text), not byte
goldens.

**Known limitations relevant to resumes**:
- No CSS emission. The output is semantic markup only. Visual styling
  (fonts, sizes, colors, multi-column layout) does not survive.
- `#set page(...)` is a warning, not a hard error (since 0.14.1), but
  page margins / page numbering / footers / headers have no effect.
- `box`, `block`, columns, tables-with-gutter: partial or inconsistent.
- Images emit as `<img>` if inline; neither of our themes ships with
  images, so not tested.
- Math: unimplemented, not relevant for resumes.

## Typst 0.14 bump — prerequisite or deferrable?

**Prerequisite. Recommendation: bump to `typst = "0.14.2"` before
implementing `--format html`.**

Evidence (POC on `research/44-html-viability`):

- With `typst = "0.13.1"` + `typst-html = "0.13.1"`, **all three themes
  fail at compile** with:

  ```
  error: page configuration is not allowed inside of containers
  ```

  This originates from 0.13's HTML emitter treating any `#set page` as
  a compiler error. Every one of our themes has a `#set page(...)` at
  the top (`text-minimal/resume.typ:60`,
  `typst-jsonresume-cv/base.typ`, `fantastic-cv/fantastic-cv.typ`).
  Stripping those would degrade the PDF output, which is an
  unacceptable trade.

- With `typst = "0.14.2"` + `typst-html = "0.14.2"`, the page rule
  becomes a warning and two of three themes produce HTML.

- `typst-html 0.13.1` exists and its crate-level API is compatible,
  but the underlying compiler's HTML path is the blocker — we cannot
  use 0.13's HTML without the `#set page` patch in 0.14.1.

**Therefore**: prompt 010 (Typst 0.14 bump) is a *hard dependency* of
prompt 011 (`--format html` implementation). The bump cannot stay
deferred.

Scope note for prompt 010: the bump is small but not zero-diff:

- `use typst::{Library, LibraryExt, World};` — the `LibraryExt` trait
  now owns `Library::default()` and `Library::builder()`.
- No other API breakage surfaced in our existing `render_world` or
  text-extraction paths during the POC build.
- `typst-assets` also goes to `^0.14.2` in lockstep.

## Per-theme compatibility — POC results

**POC setup**: `examples/html_poc.rs` on branch
`research/44-html-viability` (SHA `dbc35bd`). Copies the
`FerrocvWorld` from `src/render.rs` verbatim, switches the compile
target from `PagedDocument` to `HtmlDocument`, enables `Feature::Html`
on the library, and drives each theme in `THEMES` against
`tests/fixtures/render_full.json`. Output at `dist/research-44/`.

### `typst-jsonresume-cv` (adapter) — COMPILES, DEGRADED OUTPUT

1668-byte single-file HTML. Structure is recognizable — name in
`<title>` and `<meta name="authors">`, work and education sections as
`<div>` blocks, skill keywords as `<ul><li>`. The output has invalid
nesting (block `<p>` inside inline `<strong>`) in the date-range
rendering:

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ada Lovelace</title>
    <meta name="authors" content="Ada Lovelace">
  </head>
  <body>
    <div>
      <p><strong>Analytical Engines Ltd</strong><strong><p>Jan 1843<span style="white-space: pre-wrap">&#x20;</span></p><p><span style="white-space: pre-wrap">&#x20;</span>Nov 1852</p></strong><br><em>Programmer</em></p>
      <ul>
        <li>Published the first algorithm designed for a machine (Bernoulli numbers)</li>
        <li>Introduced the concept of a general-purpose computing device</li>
      </ul>
    </div>
    <div>
      <p><strong>University of London</strong><strong>Jan 1843</strong><br><em>Self-directed, Mathematics</em></p>
    </div>
    ...
  </body>
</html>
```

**Verdict**: renders, reads as a resume, but the HTML is malformed in
spots. Acceptable for a "known-experimental" theme. Author of prompt
011 should not write strict HTML-structure goldens for this theme.

### `fantastic-cv` (adapter) — FAILS

Compilation error:

```
error: URL must not be empty
```

Root cause: fantastic-cv's code calls `link(x.url)` in many places
(e.g. `fantastic-cv.typ:115,116,123,138,168,197,229,262,289,309`) and
the glue layer (`resume.typ:42,56,68,85,101,118,132,143,154`) defaults
missing URLs to `""`. In PDF mode `link("")` was silently tolerated; in
HTML mode it is a fatal error. This is a theme-source problem, not a
Typst bug, but fixing it requires editing the adapter's glue or
vendored source.

**Verdict**: adapter is broken for HTML until the glue defaults change
from `""` to either `none` or an explicit omission guard around each
`link(...)` call. This is deferrable — the adapter still works for
PDF, and the decision about whether to invest time patching it for
HTML or accept "PDF-only" status can be a follow-up.

### `text-minimal` (native) — COMPILES, CLEAN OUTPUT

1689-byte single-file HTML. Valid, readable, ATS-friendly:

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
  </head>
  <body>
    <p>Ada Lovelace<br>Computing Pioneer<br>ada@example.com<br>+44-20-0000-0000<br>https://example.com/ada<br>London, England, GB<br>GitHub: ada - https://github.example.com/ada<br>LinkedIn: ada-lovelace - https://linkedin.example.com/in/ada-lovelace<br></p>
    <p>Summary</p>
    <p>Mathematician and writer, chiefly known for her work on Charles Babbage's proposed mechanical general-purpose computer, the Analytical Engine.</p>
    <p>Work</p>
    <p>Programmer - Analytical Engines Ltd<br>1843-01-01 - 1852-11-27<br>Translated and annotated Menabrea's paper on the Analytical Engine; wrote the first published algorithm intended for machine execution.<br>- Published the first algorithm designed for a machine (Bernoulli numbers)<br>- Introduced the concept of a general-purpose computing device<br></p>
    <p>Education</p>
    <p>University of London<br>Self-directed, Mathematics<br>1833-01-01 - 1843-01-01<br></p>
    <p>Skills</p>
    <p>Programming (Pioneer): Analytical Engine, Algorithms<br>Mathematics (Expert): Calculus, Symbolic logic<br></p>
    ...
  </body>
</html>
```

**Verdict**: theme survives the PDF-to-HTML transition cleanly. The
`linebreak()` calls become `<br>`, `parbreak()` becomes a new `<p>`,
bold text wraps in inline elements that HTML mode omits (since the
theme produces raw strings, not nested markup). Same caveat as text
extraction: section headers are plain `<p>Summary</p>` rather than
`<h2>` — ATS tools that expect semantic headings will not see them.
Improvement opportunity for a possible dedicated `html-minimal`, but
*good enough to ship* for a first HTML release.

## CSS, font, and asset handling

**CSS**: none emitted. Typst's HTML mode does not produce a stylesheet
and has no plans to in the current phase (tracking issue #5512 defers
CSS to a later phase). The single inline style observed across all our
outputs is `<span style="white-space: pre-wrap">` for a literal
non-breaking space — trivial, already self-contained, no external
reference.

**Fonts**: not handled. No `@font-face`, no `<link rel="stylesheet">`,
no font family declaration. The browser falls back to its default.
This is consistent with the upstream docs' "emitting semantic markup"
posture. Our bundled fonts (DejaVu etc.) are irrelevant to HTML
output.

**Images**: neither theme ships with images, so not exercised. Based
on the upstream HTML API, inline `image(...)` calls would emit as
`<img>` tags, but whether that is a data URI or an external reference
depends on the source type. Since CONSTITUTION §6.1 forbids network
access at render time and the FerrocvWorld rejects packages, any image
in a theme would have to be a bundled byte slice — which Typst should
be able to emit as a data URI, but this is untested and out of scope
for the current themes.

**Single-file verdict**: **YES, single-file out of the box.** A
`grep -E 'src=|href=|link rel|font-face|@import|url\(' dist/research-44/*.html`
returns no matches. No post-processing needed. Post-processing
complexity estimate if it *were* needed: **zero** — the current output
is already single-file.

## CONSTITUTION compliance

- **§2 (no subprocess)**: HTML compilation is `typst::compile` +
  `typst_html::html`, both statically linked. Same shape as the
  existing `typst_pdf::pdf` path. No subprocess, no shell-out.
- **§6.1 (no network)**: the POC ran with the `FerrocvWorld`
  unmodified — same `PackageError::NotFound` rejection for every
  `@preview/...` import. Both successful compiles (`text-minimal`,
  `typst-jsonresume-cv`) produced HTML without any package-resolution
  attempt. HTML mode does not introduce new file-system or network
  access paths beyond what `source()` / `file()` already serve for
  PDF. The existing network-isolation test
  (`tests/render.rs::preview_package_import_is_rejected_no_network`)
  will need an HTML counterpart but the guarantee is structurally the
  same.
- **§6.4 (sandbox)**: no new `World` capabilities needed. Same trait
  impl used by PDF.
- **§3 (no feature-flag gating of HTML)**: we must enable
  `Feature::Html` on the *`Library`* to make HTML compile. That is a
  Typst-internal flag carried at library build time, *not* a Cargo
  feature of the `ferrocv` crate. The `ferrocv` user sees a plain
  `ferrocv render --format html`. §3 is about user-facing flags, so
  this is fine. (Worth documenting in the module comments of the
  eventual `compile_html` that the feature gate is a Typst internal
  and not something `ferrocv` end-users should ever have to enable.)
- **No new fonts/assets/licenses**: Typst's HTML emitter does not pull
  in new font data or license obligations. `typst-html 0.14.2` ships
  under Apache-2.0 (same as the rest of Typst), already covered in our
  `deny.toml` allowlist (no new license strings needed).

## Native html-minimal theme — needed?

**Recommendation: defer `html-minimal` to a follow-up.**

Reasoning from the POC evidence:

- `text-minimal` already produces valid, readable HTML. The output is
  not semantically *ideal* (section titles are `<p>` not `<h2>`, no
  `<header>`/`<main>`/`<section>` scaffolding, contact info is one big
  `<br>`-separated paragraph), but it is not broken.
- Shipping Phase 2's HTML release with "use `text-minimal` for HTML
  too" is CONSTITUTION §5 compliant ("simple now, iterate later"): one
  theme, one output shape, no new source.
- An eventual `html-minimal` would replace `<p>` section headers with
  `<h2>`, wrap the document in `<main>`, use `<section>` per top-level
  JSON Resume area, and emit `<a href>` for URL fields. Scope sketch
  (~100-200 lines of Typst):

  ```typ
  // html-minimal sketch:
  // - #show heading.where(level: 2): it => html.h2(it.body)
  // - per-section: html.section(html.h2[Work], ...entries...)
  // - contact block: html.header(html.h1[#name], ...)
  // - defensive dict.at(k, default: none) reads (same as text-minimal)
  // - single file in assets/themes/html-minimal/resume.typ
  ```

  The `html.*` functions are the typed HTML API added in 0.14.0.

**Signal for when to build `html-minimal`**: first real-world consumer
complains that their ATS / static-site generator chokes on the
semantic-free output. Until then, `text-minimal` doing double duty is
the right trade.

## Recommendation and plan impact

**Verdict: YELLOW — proceed with caveats.**

Prompt 010 (Typst 0.14 bump) **must run first**, and its scope is
*unchanged from the draft* — the bump goes through. Confirm in the
PR description that HTML export is the unlock.

Prompt 011 (`--format html` implementation) needs these modifications
before execution:

1. **Add `typst-html = "0.14.2"` and `typst-library = "0.14.2"`**
   dependencies (typst-html depends on typst-library, so both are
   needed as direct deps for `HtmlDocument` to be in scope cleanly).
2. **Enable `Feature::Html` in the shared library builder** — the
   existing `shared_library()` in `src/render.rs` that returns
   `Library::default()` needs a sibling (or replacement) that enables
   HTML. Suggested: make the shared library always carry
   `Feature::Html` enabled (adds no overhead for PDF compilation; the
   flag only gates the `html` namespace in the global scope).
3. **Add `compile_html(theme, data) -> Result<String, RenderError>`**
   mirroring `compile_text`. Returns `String`, not `Vec<u8>` —
   `typst_html::html` returns `SourceResult<String>`.
4. **CLI: add `Format::Html`** with default output path
   `dist/resume.html`. Default theme for `--format html` should be
   `text-minimal` (per §7's defer-html-minimal recommendation).
5. **Scope down tests for HTML**: byte-exact HTML goldens will be
   fragile across Typst minors (the experimental status guarantees
   shape churn). Recommend substring-presence + "is well-formed enough
   that `html5ever`-style parsing doesn't explode" as the assertion
   floor; avoid full-document diffs.
6. **Out of scope for prompt 011**:
   - Fixing `fantastic-cv` to compile in HTML mode. Log as a follow-up
     issue ("adapter glue: guard `link("")` for HTML compatibility").
   - Building a dedicated `html-minimal` native theme. Log as a
     follow-up issue once there's a use case.
7. **Network-isolation test**: add an HTML counterpart to the existing
   `preview_package_import_is_rejected_no_network` test in
   `tests/render.rs`. Same shape, `compile_html` entry point.
8. **Document the experimental nature** in the `compile_html` module
   comment and the CLI `--help` text — cite Typst's upstream warning
   so users aren't surprised by output shape changes across our
   dependency bumps.

Nothing else in prompts 010/011 needs to change.

## Drafted comment for issue #44

```markdown
Research on HTML export viability complete (branch
`research/44-html-viability`, SHA `dbc35bd`, POC at
`examples/html_poc.rs`). Full findings in
`research/44-html-viability.md` on main's working tree.

**Verdict: YELLOW — proceed with caveats.**

**Typst version bump is a prerequisite.** On `typst 0.13.1`,
`#set page(...)` is a fatal error in HTML mode, which kills every one
of our themes at compile. Typst 0.14.1 downgraded this to a warning,
and 0.14.2 is the current stable. Prompt 010 (the 0.14 bump) must land
before HTML implementation begins.

**Per-theme results** (all three against
`tests/fixtures/render_full.json` on `typst 0.14.2` +
`typst-html 0.14.2`):

| Theme                  | Type     | HTML result                                |
|------------------------|----------|--------------------------------------------|
| `text-minimal`         | native   | Compiles, clean single-file 1.7 KB output  |
| `typst-jsonresume-cv`  | adapter  | Compiles, degraded (invalid nesting)       |
| `fantastic-cv`         | adapter  | **Fails**: `link("")` rejected in HTML     |

`fantastic-cv` needs a glue patch (guard empty URLs) to work in HTML.
Deferrable; its PDF path keeps working. Track as a follow-up.

**Single-file**: yes, out of the box. No CSS, no fonts, no external
assets emitted. Zero post-processing needed.

**CONSTITUTION compliance**: §2 ✓ (no subprocess), §6.1 ✓ (same World,
same `PackageError::NotFound` rejection — POC confirmed), §6.4 ✓ (no
new capabilities), §3 ✓ (the required Typst `Feature::Html` is a
library-internal flag, not a user-facing Cargo feature).

**Upstream caveat**: Typst itself still marks HTML export
experimental — its reference docs say "Do not use this feature for
production use cases" and the `Feature::Html` gate explicitly warns
about API churn between releases. Our `--format html` CLI surface can
still be stable; what we're signing up for is HTML *output shape*
changes across Typst minor bumps. Recommend loose substring-based
tests over byte-for-byte HTML goldens.

**Plan impact**:
- Prompt 010 (0.14 bump): scope unchanged, runs first.
- Prompt 011 (`--format html`): add `typst-html`/`typst-library` deps,
  enable `Feature::Html` on the shared library, mirror the
  `compile_text` shape in a new `compile_html`, ship `text-minimal` as
  the default HTML theme. Descope: do not patch `fantastic-cv` for
  HTML (follow-up), do not build a dedicated `html-minimal` (follow-up
  once a real consumer asks).
```
