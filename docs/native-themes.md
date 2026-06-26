# Native themes: an authoring guide

This guide is for anyone writing a **native theme** for `ferrocv` —
either a personal theme you keep beside your own `resume.json`, or one
you want to land in the repo as a bundled default. It covers the
`render(data) -> content` contract, the shared prelude API, the
`ferrocv themes new` scaffold, and the golden-test requirement.

It is the companion to [`adapters.md`](adapters.md), which covers the
*other* kind of theme — wrappers around upstream Typst Universe
templates. If you're not sure which you want, read
["Native theme vs. adapter"](#native-theme-vs-adapter--which-should-i-write)
below first.

If you've never read [`CONSTITUTION.md`](../CONSTITUTION.md), start
there. Sections §1 (JSON Resume is canonical), §4 (two theme
interfaces), §5 (simple now), and §6 (no network at render time) are
the load-bearing ones for native-theme work; this guide refers to them
by number throughout.

## Native theme vs. adapter — which should I write?

`ferrocv` registers two kinds of themes (CONSTITUTION §4):

- **Native themes** (this guide) implement a `render(data) -> content`
  contract directly against parsed JSON Resume data, with no upstream
  to wrap. You author Typst from scratch against the shared prelude.
- **Adapters** ([`adapters.md`](adapters.md)) wrap an upstream Typst
  Universe template by mapping JSON Resume fields into the template's
  parameters. You inherit a look for free and accept breakage when
  upstream changes.

Pick a **native theme** when:

- You want a layout designed JSON-Resume-first, with no upstream
  contract to honor.
- The output target is something other than PDF — `text-minimal`
  exists precisely because frame-extracted text needs a single-column,
  bullet-free layout, and `html-minimal` targets semantic HTML.
- You want a long-lived theme that won't churn with upstream commits.

Pick an **adapter** when you want a specific look that already exists
as a Typst Universe template and you're comfortable owning
re-vendoring. See [`adapters.md`](adapters.md) for that path and its
trade-off table.

The two layers stay separable: native themes do not depend on adapter
internals, and adapter code does not leak into native themes
(CONSTITUTION §4).

## The contract

A native theme is a single Typst entrypoint. `ferrocv` hands every
theme two virtual files in the embedded `FerrocvWorld`:

- `/resume.json` — the JSON Resume document being rendered.
- `/themes/_prelude/lib.typ` — ferrocv's shared native-theme prelude.
  It's injected into **every** World — bundled, local, or even a
  scaffolded single file — so any theme can `#import` it. The path is
  the `PRELUDE_PATH` const in `src/theme.rs`; the injection happens in
  `FerrocvWorld::assemble` (`src/render.rs`).

So the skeleton of every native theme is:

```typst
#import "/themes/_prelude/lib.typ": *

#let resume = json("/resume.json")

// ...read fields through the prelude helpers, emit content...
```

The prelude is **data access only** — it ships *no* layout primitives.
That is deliberate: frame-extracted plain text and semantic HTML can't
share one `render`, so layout stays per-theme / per-format while the
defensive field-reading is shared. You write the layout; the prelude
keeps you from crashing on a sparse document.

The single invariant that drives the whole prelude: **JSON Resume
v1.0.0 has zero required fields.** `{}` is a schema-valid document.
CONSTITUTION §1 promises any schema-valid input renders, so every read
must tolerate a missing or oddly-shaped key. The helpers below exist so
you never have to sprinkle `if "x" in d` guards through your layout.

## The prelude API

The authoritative reference is the source itself —
[`assets/themes/_prelude/lib.typ`](../assets/themes/_prelude/lib.typ) —
where each helper's doc comment spells out its exact contract and edge
cases. The summary:

| Helper | Returns |
|---|---|
| `opt(d, k)` | `d.at(k)` if `d` is a dict and `k` is present, else `none`. Never errors on a non-dict `d`. |
| `nz(s)` | `none` for both `none` and `""` — collapse absent and empty so callers uniformly check `!= none`. |
| `join_present(parts, sep)` | `parts` joined with `sep`, dropping `none`/`""`. Returns `""` (not `none`) when all parts are absent — guard with `!= ""`. |
| `date_range(item)` | a range string from an item's `startDate`/`endDate`: `"2019 - 2022"` (both), `"2019 - Present"` (end absent), the bare end string (start absent — guard for this), or `none` (both absent). |
| `items(d, key)` | the array at `key`, or `()` when absent or non-array — so `for x in items(...)` and `.len() > 0` always work. |
| `ext(d, namespace)` | the `x-<namespace>` extension field on `d`, or `none`. |

A worked reading looks like the scaffold and the bundled themes:

```typst
#let basics = opt(resume, "basics")
#let name = if basics != none { nz(opt(basics, "name")) } else { none }
#if name != none { align(center, text(size: 20pt, weight: "bold")[#name]) }

#for entry in items(resume, "work") {
  // date_range tolerates either bound being missing
  entry_head(nz(opt(entry, "position")), date_range(entry))
}

// join_present is the one helper that returns "" (not none) when
// everything is absent — guard with != "", not != none:
#let loc = join_present((nz(opt(basics, "city")), nz(opt(basics, "region"))), ", ")
#if loc != "" { align(center, text(size: 9.5pt)[#loc]) }
```

### Extension fields (`x-`)

`ext(d, namespace)` is your only sanctioned door to non-stock data
(CONSTITUTION §1's `x-` mechanism). The rules, from the source doc
comment — get these wrong and you silently read `none`:

- `namespace` is the **bare** name, no `x-` prefix: `ext(d, "myorg")`
  reads `x-myorg`. Passing `ext(d, "x-myorg")` double-prefixes.
- Read sub-keys by chaining `opt`, not dot-notation or a second
  argument: `opt(ext(entry, "myorg"), "audience")`.
- The `ferrocv` namespace is **reserved** — don't read `x-ferrocv`.
  Projection (§7) consumes and strips it, so `ext(d, "ferrocv")` is
  `none` on projected output by design.

The bundled `classic` theme surfaces a `meta.x-audience` tagline as a
small worked example; the scaffold ships the same snippet commented
inline.

## Authoring workflow: `ferrocv themes new`

Don't start from a blank file. Scaffold one:

```sh
ferrocv themes new mytheme
```

This writes a new `mytheme/` directory (in the current directory, or
under `--out <dir>`) containing:

- `resume.typ` — a ready-to-edit native theme. It already `#import`s
  the prelude and renders the major JSON Resume sections (header,
  summary, experience, education, skills, projects), so it renders
  **straight away** with no edits — proof the contract works before you
  touch it.
- `golden.txt` — a placeholder golden-test stub you overwrite once the
  theme renders the way you want (see [Golden tests](#golden-tests)).

The name must be a bare directory component — ASCII letters, digits,
`-`, `_`; no leading `-`, no path separators or dots — and the command
refuses to write into an existing target rather than clobber it.

Render it by pointing `--theme` at the file:

```sh
ferrocv render resume.json --theme mytheme/resume.typ --output resume.pdf
```

Swap `--format text` or `--format html` to target the other outputs,
but note the scaffold is **tuned for PDF** (see [Format
notes](#format-notes)). From here, edit `resume.typ` freely — it's a
single self-contained file whose only dependency is the prelude. The
scaffold's comments walk through every section; the bundled
[`classic`](../assets/themes/classic/resume.typ) theme is the fuller,
every-section worked example.

## Golden tests

CONSTITUTION testing-doctrine §2: **every theme has a golden-file
test.** A golden is a committed, verbatim copy of your theme's
rendered output so a later change that alters the output shows up as a
diff you must explain. PDF bytes are too fragile across Typst patch
versions; the stable intermediate is a normalized **text extraction**.

### For a personal theme

Generate a golden by rendering a document to text and saving it next to
the theme. Run this from the directory *above* `mytheme/` (the paths
below are relative to it), and point the first argument at whatever
resume you want to pin — your own `resume.json`, or, if you don't have
one yet, any JSON Resume document (the repo ships
`tests/fixtures/render_full.json`):

```sh
ferrocv render resume.json --theme mytheme/resume.typ --format text -o mytheme/golden.txt
```

Commit `golden.txt` and diff against it in whatever test harness your
own repo uses. The scaffold's `golden.txt` stub explains this inline
(its example is written to be run from *inside* `mytheme/`, so the paths
there drop the `mytheme/` prefix — same command, different working
directory).

### For a bundled theme (contributing to ferrocv)

If you're landing a native theme into the repo as a default, it joins
the same golden harness the adapters use. Two steps:

1. **Register it in `src/theme.rs`.** All native themes are registered
   in that one file — `PRELUDE_FILE` is a private const there, so the
   registration has to live alongside it. A native theme is simpler than
   an adapter (one entrypoint plus the shared prelude, no vendored
   upstream file). Mirror the existing `CLASSIC` / `TEXT_MINIMAL`
   constants:

   ```rust
   const MY_THEME_RESUME_PATH: &str = "/themes/my-theme/resume.typ";

   pub const MY_THEME: Theme = Theme {
       name: "my-theme",
       // PRELUDE_FILE first — it's idempotent (the World injects the
       // prelude into every World regardless), but listing it keeps the
       // theme's dependency visible at its definition site.
       files: &[
           PRELUDE_FILE,
           (
               MY_THEME_RESUME_PATH,
               include_bytes!("../assets/themes/my-theme/resume.typ"),
           ),
       ],
       entrypoint: MY_THEME_RESUME_PATH,
   };
   ```

   Then append `&MY_THEME` to the `THEMES` slice.

2. **Add golden tests** in the test file that matches your output
   format:
   - **PDF** themes (like `classic`) go in
     [`tests/render_theme.rs`](../tests/render_theme.rs) via the shared
     `run_golden` helper — one test per fixture, against both
     `render_full.json` (Ada Lovelace, every field present) and
     `render_sparse.json` (Grace Hopper, optional-field degradation).
     Regenerate and lock with:

     ```sh
     UPDATE_GOLDEN=1 cargo test --test render_theme   # write goldens
     cargo test --test render_theme                   # confirm they lock in
     ```

   - **text** themes (like `text-minimal`) work the same way but live in
     [`tests/render_text.rs`](../tests/render_text.rs); use
     `--test render_text` with `UPDATE_GOLDEN`.
   - **HTML** themes (like `html-minimal`) use
     [`tests/render_html.rs`](../tests/render_html.rs), which
     deliberately does **not** keep byte-exact goldens — Typst's HTML
     export churns across versions, so it asserts on structural
     well-formedness (`<section>` elements, `mailto:` links, fixture
     name present) instead. `UPDATE_GOLDEN` does nothing there; add
     structural assertions following that file's pattern.

Inspect every golden before committing — an unexplained diff is a
regression, not a golden bump. The text/PDF harness refuses to write a
golden if the extracted text is empty or missing the fixture's name,
which keeps you from freezing garbage.

## Format notes

The scaffold targets PDF, but the native contract spans all three
output formats (CONSTITUTION §3). Layout cannot be shared across them —
that's *why* the prelude ships data access only:

- **PDF** — rich layout (grids, rules, fonts). `classic` is the
  reference. Use only bundled fonts (`Libertinus Serif`, etc.); there
  is no system-font scan, so a non-bundled family renders
  non-deterministically across hosts (CONSTITUTION §6).
- **text** — frame-walked plain text. `text-minimal` is single-column
  and bullet-free because the extractor flattens layout.
- **HTML** — Typst's typed-HTML API. `html-minimal` emits semantic
  markup.

If you want one theme to serve multiple formats well, expect to branch
layout per format; the data-reading via the prelude is the only part
that stays identical.

## Once it lands

Conventional commit prefix for a bundled native theme: `feat(themes):
add <name> theme` (user-facing). Documentation-only changes to this
guide are `docs:`. PR titles are descriptive prose, not Conventional
Commits format — release-please reads the commit log, and mirroring the
type in the title produces duplicate changelog entries on squash merge.

For the *other* kind of theme — wrapping an upstream Typst Universe
template — see [`adapters.md`](adapters.md).
