# Vendoring record — modern-cv

This directory contains a vendored, **patched** copy of the
[`DeveloperPaul123/modern-cv`] Typst resume template (canonical slug:
`ptsouchlos/modern-cv`), plus an authored glue entrypoint
(`resume.typ`) that maps JSON Resume v1.0.0 documents onto modern-cv's
helper-function call shape. The patches to `lib.typ` serve
`CONSTITUTION.md` §6.1 (no network calls at render time); the glue file
lives under §4 (adapter layer kept separable).

[`DeveloperPaul123/modern-cv`]: https://github.com/DeveloperPaul123/modern-cv

## Provenance

| Field                | Value                                                                  |
| -------------------- | ---------------------------------------------------------------------- |
| Upstream (canonical) | https://github.com/ptsouchlos/modern-cv                                |
| Upstream (legacy)    | https://github.com/DeveloperPaul123/modern-cv (redirects to canonical) |
| Commit SHA           | `dcb863f722b22255b170de9e802ece97bf5f763f`                             |
| Commit date          | 2025-09-09                                                             |
| Release tag          | `0.9.0-patch`                                                          |
| Package version      | 0.9.0                                                                  |
| Upstream source path | `lib.typ` (repo root)                                                  |
| Vendor date          | 2026-04-19                                                             |

## License

Upstream `LICENSE` is **MIT**, copyright Paul Tsouchlos, 2024.
Upstream `typst.toml` also declares MIT; there is no discrepancy
between the two. The MIT license is compatible with `ferrocv`'s
MIT-OR-Apache-2.0 dual license under the standard permissive-license
compatibility path.

We copy the upstream `LICENSE` verbatim into
[`LICENSE`](./LICENSE) in this directory so the full provenance chain
is preserved in-tree. The upstream `LICENSE` also appends Font Awesome
license notices; we keep those verbatim even though Patch A removes
all Font Awesome usages — the text is short and preserving the
upstream file lets us re-verify the copy mechanically on every
re-vendor.

## Patches applied

Three patches were applied to the upstream `lib.typ` to make it
compile under `ferrocv::render::FerrocvWorld` without reaching the
network or crashing on a missing wall clock. All three are recorded
below with before/after snippets.

### A. Remove `fontawesome` Typst Universe import and every icon call site

**Why:** `CONSTITUTION.md` §6.1 forbids network fetches at render
time, and `FerrocvWorld` rejects any `FileId` carrying a
`PackageSpec`. Upstream's first line pulled in the fontawesome
package from Typst Universe, which would trigger a package-registry
fetch on first compile.

**Before** (upstream lines 1, 6, 16-28, 134, 419, 799):

```typst
#import "@preview/fontawesome:0.6.0": *

// TODO(PT): Move to Fontawesome 7
// for now, specify Fontawesome 6
#fa-version("6")
```

```typst
#let linkedin-icon = box(fa-icon("linkedin", fill: color-darknight))
#let github-icon = box(fa-icon("github", fill: color-darknight))
// ... (11 more icon consts follow the same pattern)
#let address-icon = box(fa-icon("location-crosshairs", fill: color-darknight))
```

```typst
#let github-link(github-path) = {
  set box(height: 11pt)

  align(right + horizon)[
    #fa-icon("github", fill: color-darkgray) #link(
      "https://github.com/" + github-path,
      github-path,
    )
  ]
}
```

```typst
#if ("icon" in item) [
  #box(fa-icon(item.icon, fill: color-darknight))
]
```

**After:**

```typst
// NOTE (ferrocv vendor patch A): upstream imports the fontawesome Typst Universe
// package and calls its version helper here. Both are removed. CONSTITUTION §6.1
// forbids network fetches at render time, and the FerrocvWorld rejects any
// Typst-package import. See VENDORING.md, Patch A.
```

```typst
// const icons
// NOTE (ferrocv vendor patch A): every upstream fa-icon call below is replaced
// with empty content. Downstream code references these names, so we keep the
// `#let` bindings and let them expand to nothing. Icons are purely decorative;
// their absence leaves the surrounding layout intact. See VENDORING.md.
#let linkedin-icon = []
#let github-icon = []
// ... (11 more icon consts, each `= []`)
#let address-icon = []
```

```typst
  align(right + horizon)[
    // ferrocv vendor patch A: inline GitHub icon dropped; plain link remains.
    #box[] #link(
      "https://github.com/" + github-path,
      github-path,
    )
  ]
```

```typst
// ferrocv vendor patch A: per-item icon glyph dropped; `icon` key ignored.
#if ("icon" in item) [
  #box[]
]
```

**Behavioral impact:** resumes still render; only the decorative
icon glyphs are gone. The contact line preserves every text field
(phone, email, URL, LinkedIn username, etc.) — only the small
monochrome glyph that would have sat next to each item is removed.
The per-item `icon` key in the `author.custom` array is silently
ignored for the same reason. Layout reflow is nil because each
icon was wrapped in a `box` whose contents are now empty.

### B. Remove `linguify` Typst Universe import and inline every i18n lookup to English

**Why:** same `CONSTITUTION.md` §6.1 rationale — the linguify package
is a Typst Universe dependency. Additionally, linguify reads a
sibling `lang.toml` file on every lookup; we do not vendor that file,
which makes removing the package's call sites cleaner than stubbing
the file.

**Before** (upstream line 2 and ~8 call sites):

```typst
#import "@preview/linguify:0.4.2": *
```

```typst
let lang_data = toml("lang.toml")

let desc = if description == none {
  (
    lflib._linguify("resume", lang: language, from: lang_data).ok
      + " " + author.firstname + " " + author.lastname
  )
} else { description }

// ... and:
title: lflib._linguify("resume", lang: language, from: lang_data).ok,

// footer:
name + " · " + linguify("resume", from: lang_data)

// cover-letter variants call linguify("cover-letter"...), "attached",
// "curriculum-vitae", "sincerely", "dear", and "letter-position-pretext".
```

**After:**

```typst
// NOTE (ferrocv vendor patch B): upstream also imports the linguify Typst Universe
// package at line 2. Removed for the same reason; all i18n call sites are replaced
// with hardcoded English strings. See VENDORING.md, Patch B.
```

```typst
// NOTE (ferrocv vendor patch B): upstream read lang_data from a sibling TOML
// file here. We don't vendor that file; removing the read is consistent with
// Patch B's i18n removal. The `lang_data` parameter is still accepted by
// `__resume_footer` and `__coverletter_footer` for ABI stability, but those
// helpers no longer use it.
let lang_data = (:)

let desc = if description == none {
  // ferrocv vendor patch B: upstream composed the description from an i18n lookup
  // of the "resume" key; replaced with hardcoded English.
  (
    "Resume" + " " + author.firstname + " " + author.lastname
  )
} else { description }

// ... and:
title: "Resume",

// footer:
name + " · " + "Resume"
```

Every i18n key encountered was mapped 1:1 to plain English:

| Upstream key              | English replacement |
| ------------------------- | ------------------- |
| `resume`                  | `Resume`            |
| `cover-letter`            | `Cover Letter`      |
| `attached`                | `Attached`          |
| `curriculum-vitae`        | `Curriculum Vitae`  |
| `sincerely`               | `Sincerely`         |
| `dear`                    | `Dear`              |
| `letter-position-pretext` | `Regarding`         |

The `letter-position-pretext` key has no obvious English counterpart
in the upstream repo (upstream's own `lang.toml` renders it as
"Regarding" in en-US per the resolved string literal); we picked
"Regarding" as the most natural reading.

**Behavioral impact:** i18n is English-only. A user setting
`language: "de"` on `resume(...)` still gets "Resume" in the footer
and title slot rather than "Lebenslauf". The cover-letter helpers
behave the same way, but we don't exercise those. A future caller who
needs multilingual output can either vendor linguify locally (outside
the render path) or file an issue requesting a proper i18n shim; per
CONSTITUTION §5, that's a follow-up, not a blocker.

### C. Replace `datetime.today()` defaults with a sentinel empty string

**Why:** `FerrocvWorld::today()` deliberately returns `None` for
reproducibility (see the comment on `World::today` in
`src/render.rs`). Evaluating `datetime.today().display(...)` as a
parameter default therefore panics the moment the function is called,
which is worse than useless as a "default". Replacing the default
with `""` lets a caller who forgets to pass `date:` get a blank
footer slot instead of a cryptic crash.

**Before** (upstream lines 202, 619, 867):

```typst
#let resume(
  // ...
  date: datetime.today().display("[month repr:long] [day], [year]"),
  // ...
) = { ... }
```

(same default shape repeats in `coverletter(...)` and `hiring-entity-info(...)`).

**After:**

```typst
#let resume(
  // ...
  // NOTE (ferrocv vendor patch C): upstream default formatted the current date via
  // Typst's today() API. FerrocvWorld returns None from today() for reproducibility,
  // so evaluating that default would panic. Callers (like the ferrocv glue) are
  // expected to pass an explicit `date:`; the sentinel `""` is a harmless fallback.
  // See VENDORING.md, Patch C.
  date: "",
  // ...
) = { ... }
```

**Behavioral impact:** calling `resume(...)` without an explicit
`date:` argument now produces a blank footer-date slot instead of
panicking. The ferrocv glue always passes `date: ""` explicitly, so
the user-visible behavior is identical — this patch exists only to
make `lib.typ` safe to import under `FerrocvWorld` without forcing
every future caller to remember the `date:` argument.

## What was NOT patched

The following upstream behavior is unchanged and will follow upstream
on the next re-vendor:

- `resume(...)` body structure: paper size, margins, name/positions
  block, contact-line layout, heading show rules, footer structure,
  `author` dict schema.
- Helper functions: `resume-entry`, `resume-item`, `resume-skill-*`,
  `resume-certification`, `resume-gpa`, `justified-header`,
  `secondary-justified-header`, `github-link`.
- Default colors (`color-darknight`, `default-accent-color`, etc.)
  and default spacing.
- Cover-letter helpers (`coverletter`, `__coverletter_footer`,
  `default-closing`, `hiring-entity-info`, `letter-heading`,
  `coverletter-content`). These still compile post-patch — the
  linguify calls inside them were rewritten — but the ferrocv glue
  does not call them.

## JSON Resume mapping notes

The authored glue (`resume.typ`) makes these adapter decisions; they
live in the glue, not in `lib.typ`, so re-vendoring upstream stays a
mechanical copy.

- **`basics.name` → `author.firstname` / `author.lastname`.** JSON
  Resume carries the full name as a single string. modern-cv's
  `author` dict expects two separate components because it styles
  them differently in the big centered name (thin-weight firstname,
  bold-weight lastname). We split on the first run of whitespace:
  first token → `firstname`, remainder → `lastname`. Single-word
  names end up entirely in `firstname` with `lastname = ""`.
- **`basics.label` → `author.positions`.** JSON Resume has a single
  `label` string; modern-cv's `positions` is an array that gets
  joined with a middle-dot separator in the centered subtitle line.
  We wrap `label` in a singleton list when it's set, or pass an
  empty array otherwise.
- **Font override to `"Libertinus Serif"`.** Modern-cv's upstream
  defaults are `("Source Sans Pro", "Source Sans 3")` for body and
  `"Roboto"` for headers. Neither is present in `typst-assets::fonts()`
  (the font bundle ferrocv ships). Typst's `fallback: true` would
  still render the document, but using a substitute font that
  varies depending on exactly which fonts the bundle happens to
  include — that breaks golden-test determinism. Overriding to a
  bundled font (Libertinus Serif) keeps the golden reproducible
  across hosts. **Tradeoff:** the rendered PDF looks distinctively
  different from a stock modern-cv rendered outside ferrocv with
  Source Sans Pro installed. A future caller who wants the original
  look can file an issue for a font-override knob; CONSTITUTION §5
  defers that until a real caller asks.
- **`profile-picture: none` override.** Modern-cv's `resume(...)`
  declares `profile-picture: image` as the default — that is, the
  Typst `image` builtin *function itself*, not an image value. The
  function body then checks `if profile-picture != none`, which is
  always true for a function, and tries to pass the function into
  `block(..., profile-picture)` as a body argument. Typst 0.13
  rejects that as "unexpected argument". The glue passes
  `profile-picture: none` explicitly so the branch takes the
  no-image layout path. A future caller who wants to supply a real
  profile photo can add a knob; CONSTITUTION §5 defers that.
- **Certificates drop `issuer` and `url`.** modern-cv's
  `resume-certification` is a two-positional-arg function
  (`certification`, `date`). JSON Resume's `certificates[i].issuer`
  and `.url` have no natural slot in that shape; we silently drop
  both today. Iterate when a caller complains.
- **Cover-letter helpers are present but unused.** `lib.typ` still
  exposes `coverletter`, `hiring-entity-info`, `letter-heading`,
  etc. post-patch, but the glue's entrypoint only calls `resume(...)`.
  If a future theme wants cover-letter output it will need its own
  glue (and its own golden).

## Updating

To re-vendor from a newer upstream:

1. `curl -O https://raw.githubusercontent.com/DeveloperPaul123/modern-cv/<new-SHA>/lib.typ`
2. Copy the fetched file over `assets/themes/modern-cv/lib.typ`,
   overwriting the patched copy.
3. Re-apply Patches A, B, and C. This grep enumerates the re-apply
   surface in one pass:

    ```sh
    grep -nE "fa-icon\(|fa-version\(|linguify\(|lflib\.|toml\(\"lang|datetime\.today" \
      assets/themes/modern-cv/lib.typ
    ```

   After patching, the same grep (plus `grep -n "@preview/" ...`)
   must return nothing.
4. Refetch the upstream `LICENSE` and update it if the year or
   copyright holder changed.
5. Update the Provenance table above with the new SHA, commit date,
   release tag, and package version.
6. Adjust `resume.typ` if `resume(...)` or any helper signature
   changed upstream. Read the re-patched `lib.typ` first; the glue
   assumes the signature shape documented in this file.
7. Regenerate goldens once the golden tests land:

    ```sh
    UPDATE_GOLDEN=1 cargo test --test render_theme
    ```

   Inspect the golden diff before committing.
