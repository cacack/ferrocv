# Theme adapters: a contributor guide

This guide is for contributors who want to add a new theme adapter
to `ferrocv`. It covers what an adapter is, how the existing ones are
structured, and the step-by-step mechanics of landing a new one.

If you've never read [`CONSTITUTION.md`](../CONSTITUTION.md), start
there. Sections §1, §4, §5, and §6 are the load-bearing ones for
adapter work; this guide refers to them by number throughout.

## Adapter vs. native theme — which should I write?

`ferrocv` registers two kinds of themes (CONSTITUTION §4):

- **Adapters** wrap an upstream Typst Universe template by mapping
  JSON Resume fields into the template's parameters. The vendored
  template is treated as an opaque black box; we patch it only when
  forced to (e.g., to remove a `@preview/...` import). Breakage on
  upstream changes is accepted in exchange for visual variety.
- **Native themes** implement a `render(data) -> content` contract
  directly against parsed JSON Resume data, with no upstream to wrap.
  The `text-minimal` theme in `src/theme.rs` is the only native theme
  shipped today.

Pick **adapter** when:

- You want a specific look that already exists as a Typst Universe
  template (or any standalone Typst source you can vendor under a
  compatible license).
- You're comfortable owning re-vendoring when upstream changes.
- The upstream's input shape is close enough to JSON Resume that the
  glue stays readable (a few hundred lines, not thousands).

Pick **native theme** when:

- You want a layout designed JSON-Resume-first, with no upstream
  contract to honor.
- The output target is something other than PDF (`text-minimal` exists
  precisely because frame-extracted text needs a single-column,
  bullet-free layout).
- You want a long-lived theme that won't churn with upstream commits.

The two layers stay separable: adapter code does not leak into native
themes, and native themes do not depend on adapter internals
(CONSTITUTION §4). This guide covers adapters only; a native-theme
guide will land when there's a second native theme to draw the
pattern from.

## Anatomy of an adapter

The repo currently ships three adapters, deliberately picked to
demonstrate three different vendoring shapes. Read whichever is closest
to your case before authoring a new one.

| Adapter                                           | Shape                  | Why it's the exemplar                                                                                  |
| ------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------ |
| [`fantastic-cv`](../assets/themes/fantastic-cv/)             | Pure glue, light patch | Upstream needed only one compatibility patch (`link("")` guard); everything else is glue in `resume.typ`. |
| [`typst-jsonresume-cv`](../assets/themes/typst-jsonresume-cv/) | Glue + optional-field shim | Upstream read a path the World doesn't serve, plus several dict lookups that crashed on sparse JSON Resume documents. |
| [`modern-cv`](../assets/themes/modern-cv/)                   | Heavy patch            | Upstream pulled two `@preview/...` packages and read the system clock; required removing both and i18n-flattening to English. |

Every adapter directory has the same shape:

```
assets/themes/<name>/
├── LICENSE          # upstream license, copied verbatim
├── VENDORING.md     # provenance + patch record
├── <upstream>.typ   # vendored upstream source (possibly patched)
└── resume.typ       # ferrocv-authored glue entrypoint
```

The vendored upstream file (`<upstream>.typ`) is byte-for-byte from
upstream when possible; any deviation is documented patch-by-patch in
`VENDORING.md`. The `resume.typ` glue is always ferrocv-authored and
contains all JSON Resume → upstream-template field-mapping logic. This
split is what makes re-vendoring a mechanical operation: you copy the
new upstream over `<upstream>.typ`, re-apply the documented patches,
and the glue continues to work as long as the upstream signatures
didn't change.

## Step-by-step: adding a new adapter

### 1. Vet the upstream

Before vendoring anything:

- **License compatibility.** `ferrocv` is dual-licensed MIT-OR-Apache-2.0.
  Permissive licenses (MIT, BSD-2/3, Apache-2.0) and public-domain
  dedications (Unlicense, CC0) work; copyleft licenses (GPL, AGPL) do
  not. If `typst.toml` and the `LICENSE` file disagree (this happens —
  see `assets/themes/fantastic-cv/VENDORING.md`), prefer the actual
  `LICENSE` file's text and document the discrepancy.
- **Network audit.** Grep the upstream source for `@preview/` and any
  `import` of an external package. Each match is a patch you'll have
  to write and maintain (CONSTITUTION §6.1: no network calls at render
  time; the World rejects every `PackageSpec`). A handful is fine; a
  pervasive icon library is a maintenance burden.
- **Wall-clock audit.** Grep for `datetime.today(`. The World's
  `today()` returns `None` for reproducibility (see `src/render.rs`),
  so any default that calls `datetime.today().display(...)` will panic
  the moment the function is invoked. Each call site needs a sentinel
  default (typically `""`) and explicit caller-supplied dates.
- **Pin a commit SHA.** Tags move; SHAs don't. Record the commit SHA
  in `VENDORING.md`; that's what re-vendoring will diff against.

### 2. Vendor the upstream

Create `assets/themes/<name>/` and copy in:

1. The minimum upstream Typst source needed to render. Some templates
   ship a `lib.typ` re-export shim purely for Typst Universe packaging
   — skip those and import the real source file directly from the
   glue.
2. The upstream `LICENSE` file, verbatim, as `LICENSE`.
3. A new `VENDORING.md` (next step).

Do **not** vendor: README files, examples, screenshots, build scripts,
or anything not loaded at compile time. Keeping the vendor footprint
minimal makes re-vendoring cheaper.

### 3. Write `VENDORING.md`

`VENDORING.md` is the contract that makes re-vendoring mechanical.
Use one of the existing files as a template. Required sections:

- **Provenance table** — upstream URL, commit SHA, commit date,
  package version (if applicable), upstream source path, vendor date.
- **License** — what license upstream ships under, why it's compatible
  with `ferrocv`'s dual license, any discrepancies you found.
- **Patches applied** — every deviation from upstream, each with a
  before/after snippet, the line numbers affected, and the
  constitutional or technical reason. Future-you will re-apply these
  on every re-vendor.
- **What was NOT patched** — explicit list of upstream behavior we
  inherit, so a re-vendor reviewer knows what to check.
- **JSON Resume mapping notes** — adapter decisions that live in the
  glue (field renames, pluralization, fields with no JSON Resume
  equivalent, fields JSON Resume has in two places).
- **Updating** — a step-by-step recipe for re-vendoring from a newer
  upstream. The grep commands you used to audit in step 1 belong here.

### 4. Author the glue (`resume.typ`)

The glue is the only file that touches `r.basics`, `r.work`, etc.
directly. The vendored upstream file does not parse JSON Resume — it
only sees the dictionary shapes the glue hands it.

Three patterns every glue file follows:

**Read JSON Resume from the World.** The `FerrocvWorld` always serves
the resume bytes at `/resume.json`. Other paths will not resolve.

```typst
#let r = json("/resume.json")
```

**Optional-field shim.** JSON Resume v1.0.0 has zero required fields —
`{}` is a schema-valid document. Every read uses `.at(..., default: ...)`
and every section guards on `"<key>" in r and r.<key> != none and
r.<key>.len() > 0`. CONSTITUTION §1 promises that any schema-valid
input renders; a `KeyError` on a missing field violates that promise.

```typst
#let basics = r.at("basics", default: (:))
#let location = basics.at("location", default: (:))
#let city = location.at("city", default: "")
```

**Field mapping isolated to the glue.** Pluralization mismatches
(`work` → `works`), renames (`summary` → `description`), and fields
with no JSON Resume equivalent (filled with `""` or `()`) all live
here. Don't push them into the vendored file — that fork makes
re-vendoring painful. See `assets/themes/fantastic-cv/resume.typ` for
the exhaustive worked example.

### 5. Register the theme in `src/theme.rs`

The registry is a static slice; no builder, no `HashMap`. Three
additions in `src/theme.rs`:

1. A `<NAME>_PREFIX` private const for the virtual-path prefix
   (centralizes the path so `files` and `entrypoint` can't drift):

   ```rust
   const MY_THEME_PREFIX: &str = "/themes/my-theme";
   ```

2. A `pub const MY_THEME: Theme` with `include_bytes!` for every file
   the entrypoint imports (transitively). Use `concat!` against the
   prefix const so a typo becomes a build error:

   ```rust
   pub const MY_THEME: Theme = Theme {
       name: "my-theme",
       files: &[
           (
               concat!("/themes/my-theme", "/upstream.typ"),
               include_bytes!("../assets/themes/my-theme/upstream.typ"),
           ),
           (
               concat!("/themes/my-theme", "/resume.typ"),
               include_bytes!("../assets/themes/my-theme/resume.typ"),
           ),
       ],
       entrypoint: concat!("/themes/my-theme", "/resume.typ"),
   };
   ```

3. Append a reference to the `THEMES` slice:

   ```rust
   pub const THEMES: &[&Theme] = &[
       // ...existing entries...
       &MY_THEME,
   ];
   ```

The CLI's `--theme` flag will pick up the new entry automatically;
`find_theme("my-theme")` does a linear scan over `THEMES`.

### 6. Add golden-file tests

CONSTITUTION testing.2: every theme has a golden-file test. The
deterministic intermediate is normalized text from `pdf-extract` —
PDF bytes are too fragile across Typst patch versions, but extracted
text catches the regressions a golden is meant to catch (missing
sections, re-ordered fields, lost data).

Add two test functions in `tests/render_theme.rs`, one per fixture:

```rust
#[test]
fn my_theme_renders_ada_lovelace_to_expected_text() {
    run_golden(
        "my-theme",
        "tests/fixtures/render_full.json",
        "tests/goldens/my-theme.txt",
        "Ada Lovelace",
    );
}

#[test]
fn my_theme_renders_grace_hopper_sparse_to_expected_text() {
    run_golden(
        "my-theme",
        "tests/fixtures/render_sparse.json",
        "tests/goldens/my-theme-sparse.txt",
        "Grace Hopper",
    );
}
```

Both fixtures matter. `render_full.json` (Ada Lovelace) pins the
"every field present" output shape. `render_sparse.json` (Grace
Hopper) pins the optional-field shim — a silent regression in how
missing fields degrade becomes a committed diff.

Generate the goldens, inspect them, and commit:

```sh
UPDATE_GOLDEN=1 cargo test --test render_theme
```

Re-run without `UPDATE_GOLDEN` to confirm the goldens lock in:

```sh
cargo test --test render_theme
```

If the extracted text is empty, garbled, or missing the fixture's
name, the test refuses to write the golden — that's the sanity check
that prevents freezing garbage.

## Pitfalls

A short list of things that will bite you, in rough order of how
often they bite:

- **`link("")` is fatal in Typst 0.14+.** Any code path that calls
  `link(x.url)` where `x.url` might be an empty string will die at
  compile. Guard the call itself, not afterward (see
  `assets/themes/fantastic-cv/VENDORING.md` patch rationale).
- **`@preview/...` imports are rejected by the World.** Even if the
  package is harmless in isolation, the import triggers a package
  resolver that the World refuses to implement (CONSTITUTION §6.1).
  Either remove the import and inline a stub, or pick a different
  upstream.
- **`datetime.today()` returns `None`.** Defaults like
  `date: datetime.today().display(...)` crash on first call. Replace
  with `date: ""` and let the glue pass an explicit value (or accept
  a blank slot).
- **System fonts are not available.** `ferrocv` ships fonts via the
  `typst-assets` crate (DejaVu Sans Mono, Libertinus Serif, New
  Computer Modern, etc.) — there is no system-font scan. If upstream
  defaults to a font that isn't bundled, override to one that is, or
  golden tests will be non-deterministic across hosts. See
  `assets/themes/modern-cv/VENDORING.md` for the worked example
  (Libertinus Serif substituted for Source Sans Pro / Roboto).
- **Schema-valid is not the same as realistic.** JSON Resume's v1.0.0
  schema makes every field optional. Test against `render_sparse.json`,
  not just `render_full.json`, before claiming the adapter handles
  arbitrary input.

## Once it lands

Conventional commit prefix for theme work: `feat(themes): add <name>
adapter` (user-facing) or `chore(themes): ...` (e.g., re-vendoring).
PR titles are descriptive prose, not Conventional Commits format —
release-please reads the commit log, and mirroring the type in the
title produces duplicate changelog entries on squash merge.

When the upstream releases a new version, re-vendoring is:

1. Copy the new upstream file over `<upstream>.typ`.
2. Re-apply every patch in `VENDORING.md` (the `Updating` section
   should walk you through it; if it doesn't, tighten it).
3. Re-run `UPDATE_GOLDEN=1 cargo test --test render_theme` and
   inspect the diff before committing.

If the diff is small and explicable, you're done. If it isn't, the
upstream signature probably changed and the glue needs an update too.
