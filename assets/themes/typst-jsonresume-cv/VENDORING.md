# Vendoring record — `typst-jsonresume-cv`

This directory contains a vendored copy of the `basic-resume` theme from
[`fruggiero/typst-jsonresume-cv`], patched to satisfy the constitutional
no-network constraint (see `CONSTITUTION.md` §6.1) and to read the JSON
Resume bytes from the path `ferrocv`'s Typst `World` serves them under.

[`fruggiero/typst-jsonresume-cv`]: https://github.com/fruggiero/typst-jsonresume-cv

See also: [`../VENDORING_CHECKLIST.md`](../VENDORING_CHECKLIST.md) —
the shared runbook for initial vendoring, re-vendoring, and Typst
bumps.

## Provenance

| Field       | Value                                                                 |
| ----------- | --------------------------------------------------------------------- |
| Upstream    | https://github.com/fruggiero/typst-jsonresume-cv                      |
| Commit SHA  | `cc45b46336421126d64ec956ccbd9be8b170e7f9`                            |
| Commit date | 2026-03-10 (upstream `main`)                                          |
| Theme path  | `themes/basic-resume/` in upstream                                    |
| Vendored    | 2026-04-18                                                            |

## License

The upstream repo does **not** ship a LICENSE file of its own. Its README
states the project "maintains the same license as the original template,"
referring to [`stuxf/basic-typst-resume-template`], which is released
under **[The Unlicense]** (public-domain dedication).

[`stuxf/basic-typst-resume-template`]: https://github.com/stuxf/basic-typst-resume-template
[The Unlicense]: https://unlicense.org/

Because `ferrocv` is MIT-or-Apache-2.0 licensed, and the Unlicense is a
public-domain dedication (compatible with redistribution under any
license), vendoring is safe. We copy the Unlicense text into
[`LICENSE`](./LICENSE) in this directory so the full provenance chain is
preserved in-tree.

## Patches applied

Three patches were applied to the upstream theme source to make it
compile under `ferrocv::render::FerrocvWorld` against arbitrary
schema-valid JSON Resume documents. All three are recorded below with
before/after snippets.

### 1. `base.typ`: remove `@preview/scienceicons` import

**Why:** `CONSTITUTION.md` §6.1 forbids network fetches at render time,
and `FerrocvWorld` rejects any `FileId` carrying a `PackageSpec`. The
upstream import of `@preview/scienceicons:0.0.6` would trigger a
package-registry fetch on first compile.

**Before** (upstream line 1 and line 169):

```typst
#import "@preview/scienceicons:0.0.6": orcid-icon
```

```typst
contact-item(orcid, prefix: [#orcid-icon(color: rgb("#AECD54"))orcid.org/], link-type: "https://orcid.org/"),
```

**After:**

```typst
// NOTE (ferrocv vendor patch): the upstream `#import "@preview/scienceicons:0.0.6": orcid-icon`
// has been removed. CONSTITUTION §6.1 forbids network fetches at render time, and the
// FerrocvWorld rejects `@preview/...` packages. The single call site below was rewritten
// to drop the icon glyph. See VENDORING.md for the full diff.
```

```typst
// ferrocv vendor patch: scienceicons removed (CONSTITUTION §6.1).
// The ORCID icon glyph is dropped; the textual "orcid.org/<id>" link is kept.
contact-item(orcid, prefix: "orcid.org/", link-type: "https://orcid.org/"),
```

**Behavioral impact:** resumes that set an ORCID id still render, and
the `orcid.org/<id>` textual link still appears in the contact line.
Only the small green ORCID logo glyph is gone. No layout reflow —
`orcid-icon` was inline content inside a `prefix:` argument and its
absence does not change the surrounding structure.

### 2. `resume.typ`: read `/resume.json` instead of `../../output/...`

**Why:** the upstream path assumes the Node build layout (`index.js`
copies the user's resume into `output/resume-data.json`). In `ferrocv`,
the `FerrocvWorld` serves the JSON Resume bytes at the virtual path
`/resume.json`. Same filename convention as every other ferrocv Typst
source (see the smoke tests in `tests/render.rs`).

**Before** (upstream line 19):

```typst
#let r = json("../../output/resume-data.json")
```

**After:**

```typst
// ferrocv vendor patch: upstream read from "../../output/resume-data.json" (the Node
// build script copies the user's resume.json there). In ferrocv, the FerrocvWorld
// serves the JSON Resume bytes at the virtual path "/resume.json". See VENDORING.md.
#let r = json("/resume.json")
```

**Behavioral impact:** none — the theme reads the same JSON Resume
fields from the same parsed structure. Only the lookup path changes.

### 3. `resume.typ`: optional-field shim

**Why:** JSON Resume v1.0.0 has zero required fields — the schema
allows `{}` as a valid document — but the upstream template reads
several fields directly (`r.meta.language`, `r.basics.location.city`,
`r.basics.location.region`, `r.basics.email`, `r.basics.phone`) and
dereferences the top-level `work`/`projects`/`education` arrays
without an `in`-check. Any schema-valid document omitting one of
those fields produces a Typst dictionary-lookup error at render time,
directly conflicting with `CONSTITUTION.md` §1 ("JSON Resume is the
canonical input, unmodified"). This patch wraps every such read in
`.at(..., default: ...)` and guards the top-level section checks
with `"<key>" in r` so that any schema-valid resume renders.

**Before** (upstream shape):

```typst
#let getProfile(resume, network) = {
  let profile = none
  if "profiles" in resume.basics and resume.basics.profiles != none {
    ...

#let lang = r.meta.language
#let name = r.basics.name
#let address = r.basics.location.city + ", " + r.basics.location.region
#let emailAddress = r.basics.email
#let phoneNumber = r.basics.phone

#if show_work and r.work != none and r.work.len() > 0 { ... }
#if show_projects and r.projects != none and r.projects.len() > 0 { ... }
#if show_education and r.education != none and r.education.len() > 0 { ... }
```

**After:**

```typst
#let getProfile(resume, network) = {
  let profile = none
  let basics = resume.at("basics", default: (:))
  if "profiles" in basics and basics.profiles != none {
    ...

#let basics = r.at("basics", default: (:))
#let meta = r.at("meta", default: (:))
#let location = basics.at("location", default: (:))

#let lang = meta.at("language", default: "en")
#let name = basics.at("name", default: "")
#let city = location.at("city", default: "")
#let region = location.at("region", default: "")
#let address = if city != "" and region != "" {
  city + ", " + region
} else if city != "" {
  city
} else {
  region
}
#let emailAddress = basics.at("email", default: "")
#let phoneNumber = basics.at("phone", default: "")

#if show_work and "work" in r and r.work != none and r.work.len() > 0 { ... }
#if show_projects and "projects" in r and r.projects != none and r.projects.len() > 0 { ... }
#if show_education and "education" in r and r.education != none and r.education.len() > 0 { ... }
```

**Behavioral impact:** a full-field resume renders identically — the
`render_theme` golden test confirms byte-for-byte stability of the
extracted text against the committed `render_full.json` fixture.
Sparse resumes that previously crashed now render with sensible
defaults: missing language falls back to `"en"` (so the English
section titles apply), missing contact fields produce empty strings
(which `contact-item` already filters out of the contact line), and
missing location components collapse (`city, region` → just `city`
or just `region` or empty).

**What the shim does not address:** fields inside work/education/
projects/skills entries (`job.name`, `job.position`,
`education_item.institution`, `education_item.studyType`,
`education_item.area`, `skill.name`, `skill.keywords`, etc.) are
still read unconditionally inside `base.typ`'s section builders.
JSON Resume v1.0.0 treats these as optional, but in practice every
realistic entry carries them. If a future bug report surfaces a
schema-valid document that's sparse *inside* an entry, extend the
shim (or fix it at the ferrocv Rust layer by normalizing the JSON
before handing it to Typst) at that point.

## What was NOT patched

- `base.typ` `resume` function signature and behavior — untouched.
- Typst parameter shape (`author`, `location`, `email`, etc.) —
  untouched.
- Section builders (`work`, `edu`, `projects`,
  `cumulativeCertSkillsInterests`) — untouched.
- Multilingual `section_titles` dictionary — untouched.

## Empty-URL safety audit

**Audit date:** 2026-04-20. **Outcome:** safe by construction; no
patch required.

Typst `link("")` is a fatal error in 0.14.x, and JSON Resume v1.0.0
makes every `url` field optional, so every vendored adapter gets an
empty-URL audit on vendor and on re-vendor. See
[`../VENDORING_CHECKLIST.md`](../VENDORING_CHECKLIST.md) §2 for the
shared procedure; this section records the outcome for this theme.

```sh
$ grep -n "link(" assets/themes/typst-jsonresume-cv/*.typ
assets/themes/typst-jsonresume-cv/base.typ:152:        link(link-type + value)[#(prefix + value)]
```

One and only one `link(...)` call site, inside the `contact-item`
helper defined at `base.typ:149`:

```typst
let contact-item(value, prefix: "", link-type: "") = {
  if value != "" {
    if link-type != "" {
      link(link-type + value)[#(prefix + value)]
    } else {
      value
    }
  }
}
```

The outer `if value != ""` guard means `link()` is never invoked with
an empty string. The glue `resume.typ` populates every caller via
`.at("email", default: "")`, `.at("phone", default: "")`,
`.at("github", default: "")`, etc. — empty-string defaults flow
cleanly through the guard. The top-level `basics.url` field is
handled via a separate `website != none` check upstream and never
reaches `contact-item`.

`resume.typ` itself has zero `link(` call sites (grep-verified), so
the audit surface is exactly the one call above.

**Regression test:** `render_accepts_sparse_schema_valid_resume` in
`tests/render_cli.rs` renders `tests/fixtures/render_sparse.json`
(only `basics.name` plus one `work` entry, no URLs) through this
theme to PDF and asserts the output is a valid PDF. Any future patch
that weakens the `if value != ""` guard will fail that test.

**Re-vendor obligation:** re-run the grep above after every upstream
bump. If a new `link(` call site appears, trace its argument back to
its source; if the source can ever be an empty string, apply an
empty-URL guard before committing.

## Updating

To re-vendor from a newer upstream:

1. `git clone --depth 1 https://github.com/fruggiero/typst-jsonresume-cv /tmp/typst-jsonresume-cv`
2. Copy `themes/basic-resume/base.typ` and `themes/basic-resume/resume.typ` over
   the files in this directory.
3. Re-apply the two patches above. A quick grep for `@preview/` and
   `../../output/` in the copied files catches both.
4. Update the commit SHA and vendor date in this file.
5. Regenerate goldens and inspect the diff before committing:

    ```sh
    UPDATE_GOLDEN=1 cargo test --test render_theme
    ```
