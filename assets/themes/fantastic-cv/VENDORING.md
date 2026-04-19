# Vendoring record — `fantastic-cv`

This directory contains a vendored copy of [`austinyu/fantastic-cv`]'s
Typst source, plus a ferrocv-authored glue entrypoint that maps
JSON Resume v1.0.0 data to fantastic-cv's argument shapes. Vendoring
satisfies the constitutional no-network constraint (see
`CONSTITUTION.md` §6.1), and keeping the mapping isolated in
`resume.typ` preserves the adapter/native separation (see `CONSTITUTION.md`
§4).

[`austinyu/fantastic-cv`]: https://github.com/austinyu/fantastic-cv

## Provenance

| Field                | Value                                                  |
| -------------------- | ------------------------------------------------------ |
| Upstream             | https://github.com/austinyu/fantastic-cv               |
| Commit SHA           | `37e4ab219abbc8fa9419b7e71a8acddfafa7dfa9`             |
| Commit date          | 2025-05-14 (upstream `main`)                           |
| Package version      | 0.1.0 (first and only release, published 2025-05-15)   |
| Upstream source path | `src/fantastic-cv.typ`                                 |
| Vendor date          | 2026-04-19                                             |

We do **not** vendor the upstream `src/lib.typ`; it is a 3-line
re-export shim whose sole purpose is Typst Universe packaging. Our
glue imports directly from `fantastic-cv.typ`.

## License

The upstream repository ships two slightly inconsistent license
declarations:

- `typst.toml` declares the license as **MIT**.
- The committed `LICENSE` file is [**The Unlicense**] (a public-domain
  dedication).

[**The Unlicense**]: https://unlicense.org/

We vendor the actual `LICENSE` file's text as [`LICENSE`](./LICENSE) in
this directory because that's the license text upstream actually ships
alongside the source. The `typst.toml` `MIT` string appears to be a
packaging-time mis-statement.

The discrepancy does not affect our ability to redistribute:
public-domain dedications under the Unlicense explicitly permit copy,
modify, publish, and distribute under any terms, which makes Unlicense
compatible with `ferrocv`'s **MIT OR Apache-2.0** dual license. This is
the same precedent we relied on when vendoring
[`typst-jsonresume-cv`](../typst-jsonresume-cv/VENDORING.md) (whose
upstream also traces back to an Unlicense root).

Upstream LICENSE for reference:
<https://github.com/austinyu/fantastic-cv/blob/37e4ab219abbc8fa9419b7e71a8acddfafa7dfa9/LICENSE>

## Patches applied

**One compatibility patch**: every `link(x.url)` call in
`fantastic-cv.typ` was restructured to guard the empty-URL case *before*
`link()` is called, rather than upstream's "call `link()` then maybe
rebind" pattern.

Why: Typst `link("")` is a fatal error in 0.14.x (`error: URL must not
be empty`). Upstream's original pattern —

```typst
let main = link(education.url)[#education.institution]
if education.url.len() == 0 {
  main = education.institution
}
```

— evaluates `link("")` eagerly before the post-hoc guard ever runs, so
any schema-valid JSON Resume that omits a `url` on an entry dies at
compile. JSON Resume v1.0.0 makes every `url` field optional, so
real-world resumes trigger this constantly.

The patch pattern applied at every call site (11 sites in total) is
semantically identical but guards the `link()` call itself:

```typst
let main = if education.url.len() == 0 { education.institution } else { link(education.url)[#education.institution] }
```

Affected line ranges (after patching): 115, 116, 123, 138, 165, 191,
192, 217, 247, 271, 288 — i.e. every `link(...)` call that takes a
URL field out of the JSON Resume data.

The patch is minimal, preserves upstream's intent exactly, and is
forward-compatible with any future upstream fix (if upstream adopts
the same restructure, re-vendoring becomes a byte-for-byte copy again).

All other JSON-Resume → fantastic-cv field mapping and the
optional-field shim live in the adjacent `resume.typ` glue entrypoint,
as before.

The upstream source does **not** import any `@preview/...` packages at
the pinned SHA, so there is no network-fetch patch to apply (contrast
with the `typst-jsonresume-cv` vendor). A `grep -n "@preview/"
fantastic-cv.typ` returns no matches — re-check this on every re-vendor.

## What was NOT patched

The following are deliberately upstream-as-is:

- `fantastic-cv.typ` function signatures (`config`, `render-basic-info`,
  `render-education`, `render-work`, `render-project`, `render-volunteer`,
  `render-award`, `render-certificate`, `render-publication`,
  `render-custom`).
- The `config` show-rule body (font/page/heading styles).
- All `render-*` builder bodies — entry layout, `_section` helper,
  `_format_dates` and `_entry_heading` internals.
- Default font ("New Computer Modern"), default font size (10pt),
  default paper ("a4"), default margins (0.5in all sides), and
  default accent color (`#26428b`).

The `resume.typ` glue invokes `config.with()` with zero overrides —
CONSTITUTION §5 says a second caller is the trigger to expose knobs,
not the first.

## JSON Resume mapping notes

The ferrocv glue `resume.typ` handles these shape frictions without
touching `fantastic-cv.typ`:

- **Key pluralization:** JSON Resume singular keys (`work`, `education`,
  `volunteer`) map to fantastic-cv plural argument names (`works`,
  `educations`, `volunteers`).
- **Field renames:** JSON Resume `work[].summary` →
  fantastic-cv `work.description`.
- **Fields fantastic-cv expects that JSON Resume lacks:**
  `work[].location`, `education[].location`, `volunteer[].location`
  (all defaulted to `""`), `project[].source_code` (defaulted to `""`).
- **Fields JSON Resume has in two places:** `project[].roles` is read
  first; `project[].keywords` is the fallback (both are schema-legal).
- **Skills:** fantastic-cv has no dedicated skills builder, so JSON
  Resume `skills` surface via `render-custom` as a "Skills" section.
  Empty or missing `skills` emits no section.

## Updating

To re-vendor from a newer upstream:

1. Fetch the upstream `src/fantastic-cv.typ` at the new commit SHA
   (e.g., `curl -O
   https://raw.githubusercontent.com/austinyu/fantastic-cv/<SHA>/src/fantastic-cv.typ`).
2. Copy it over the file in this directory, overwriting byte-for-byte.
3. **Re-apply the empty-URL-guard patch** documented under "Patches
   applied" above. `grep -n "link(" fantastic-cv.typ` enumerates the
   call sites; each must be guarded as shown. If upstream has adopted
   the same restructure, the grep result will already match the
   patched shape and this step is a no-op.
4. Update the Commit SHA, Commit date, Package version, and Vendor
   date rows in the Provenance table above.
5. Adjust `resume.typ` if any of fantastic-cv's `render-*` signatures
   changed. `grep -n "^#let \(config\|render-\)" fantastic-cv.typ`
   enumerates the current signatures; `grep -n "render-" resume.typ`
   enumerates every call site.
6. Re-verify no new `@preview/...` imports crept in:
   `grep -n "@preview/" fantastic-cv.typ` must return no matches.
7. Regenerate goldens: `UPDATE_GOLDEN=1 cargo test --test render_theme`
   and inspect the diffs before committing.
