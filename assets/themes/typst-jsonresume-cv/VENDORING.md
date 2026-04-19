# Vendoring record — `typst-jsonresume-cv`

This directory contains a vendored copy of the `basic-resume` theme from
[`fruggiero/typst-jsonresume-cv`], patched to satisfy the constitutional
no-network constraint (see `CONSTITUTION.md` §6.1) and to read the JSON
Resume bytes from the path `ferrocv`'s Typst `World` serves them under.

[`fruggiero/typst-jsonresume-cv`]: https://github.com/fruggiero/typst-jsonresume-cv

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

Two patches were applied to the upstream theme source to make it
compile under `ferrocv::render::FerrocvWorld`. Both are recorded below
with before/after snippets.

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

## What was NOT patched

- `base.typ` `resume` function signature and behavior — untouched.
- Typst parameter shape (`author`, `location`, `email`, etc.) —
  untouched.
- Section builders (`work`, `edu`, `projects`,
  `cumulativeCertSkillsInterests`) — untouched.
- Multilingual `section_titles` dictionary — untouched.

## Updating

To re-vendor from a newer upstream:

1. `git clone --depth 1 https://github.com/fruggiero/typst-jsonresume-cv /tmp/typst-jsonresume-cv`
2. Copy `themes/basic-resume/base.typ` and `themes/basic-resume/resume.typ` over
   the files in this directory.
3. Re-apply the two patches above. A quick grep for `@preview/` and
   `../../output/` in the copied files catches both.
4. Update the commit SHA and vendor date in this file.
5. Run the render tests (`cargo test --test render`) and, once the
   golden test lands from issue #12, its golden fixture.
