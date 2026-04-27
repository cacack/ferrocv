# Vendoring record — `basic-resume`

This directory contains a vendored copy of [`stuxf/basic-typst-resume-template`]'s
Typst source (the package published on Typst Universe as `basic-resume`),
plus a ferrocv-authored glue entrypoint that maps JSON Resume v1.0.0 data
to basic-resume's argument shapes. Vendoring satisfies the constitutional
no-network constraint (see `CONSTITUTION.md` §6.1), and keeping the
mapping isolated in `resume.typ` preserves the adapter/native separation
(see `CONSTITUTION.md` §4).

[`stuxf/basic-typst-resume-template`]: https://github.com/stuxf/basic-typst-resume-template

See also: [`../VENDORING_CHECKLIST.md`](../VENDORING_CHECKLIST.md) —
the shared runbook for initial vendoring, re-vendoring, and Typst
bumps.

## Provenance

We vendor from the [`typst/packages`] mirror rather than the upstream
GitHub repo because Typst Universe's published archive is the
authoritative shape (release tags on the upstream repo do not always
match the Universe-published `<name>-<version>.tar.gz` byte-for-byte).

[`typst/packages`]: https://github.com/typst/packages

| Field                | Value                                                                                          |
| -------------------- | ---------------------------------------------------------------------------------------------- |
| Upstream             | https://github.com/stuxf/basic-typst-resume-template                                           |
| Vendored from        | https://github.com/typst/packages/tree/main/packages/preview/basic-resume/0.2.9                |
| Commit SHA           | `d22237083846d393b1f3693ab7652b072827451f` (typst/packages, last touch on `basic-resume/0.2.9`) |
| Commit date          | 2025-09-29                                                                                     |
| Package version      | 0.2.9                                                                                          |
| Upstream source path | `packages/preview/basic-resume/0.2.9/src/resume.typ`                                           |
| Vendor date          | 2026-04-27                                                                                     |

We do **not** vendor the upstream `src/lib.typ`; it is a 4-line
re-export shim whose sole purpose is Typst Universe packaging. Our
glue imports directly from `basic-resume.typ` (renamed from upstream's
`src/resume.typ` to avoid colliding with the ferrocv glue file's name).

## License

Upstream's `LICENSE` and its `typst.toml` `license` field both declare
[**The Unlicense**] — no discrepancy to record.

[**The Unlicense**]: https://unlicense.org/

The Unlicense is a public-domain dedication that explicitly permits
copy, modify, publish, and distribute under any terms, which makes it
compatible with `ferrocv`'s **MIT OR Apache-2.0** dual license. The
file-level [`LICENSE`](./LICENSE) is duplicated from upstream verbatim.

Upstream LICENSE for reference:
<https://github.com/typst/packages/blob/main/packages/preview/basic-resume/0.2.9/LICENSE>

## Patches applied

**One §6.1 patch**: the upstream `src/resume.typ` opens with

```typst
#import "@preview/scienceicons:0.1.0": orcid-icon
```

and uses `orcid-icon(...)` once, in the contact-line ORCID prefix:

```typst
contact-item(orcid, prefix: [#orcid-icon(color: rgb("#AECD54"))orcid.org/], link-type: "https://orcid.org/"),
```

CONSTITUTION §6.1 forbids `@preview/...` imports — `FerrocvWorld`
rejects any `FileId` carrying a `PackageSpec` rather than fetching
it. Both the import and the call site were rewritten:

```typst
// after patching
contact-item(orcid, prefix: "ORCID: orcid.org/", link-type: "https://orcid.org/"),
```

The visual difference is one missing icon glyph in the contact line
when an ORCID is present; the textual content (the literal "ORCID:"
label and the `orcid.org/<id>` link) is unchanged. `text-minimal` and
`html-minimal` users who want a fully glyph-free document already have
those native themes; `basic-resume` users who want richer iconography
can vendor a different theme.

If upstream drops the `@preview/scienceicons` dependency in a later
release (e.g. by inlining an SVG or removing the icon entirely), the
re-vendor becomes a byte-for-byte copy and this patch can be retired.

## Empty-URL safety audit

Per the [shared checklist](../VENDORING_CHECKLIST.md) §2, every
`link(...)` call site was traced back to its source. Audit results
(line numbers in `basic-resume.typ` after patching):

| Line | Call                                  | Guarded by                                  |
| ---- | ------------------------------------- | ------------------------------------------- |
| 83   | `link(link-type + value)`             | `if value != ""` in `contact-item` (L81)    |
| 202  | `link("https://" + url)`              | `if url != "" and dates != ""`              |
| 204  | `link("https://" + url)`              | `if url != "" and dates != ""`              |
| 209  | `link("https://" + url)`              | `if dates == "" and url != ""`              |
| 226  | `link("https://" + url)`              | `if url != ""` in `certificates`            |

All five sites are guarded; no empty-URL patch is required. The
sparse-fixture regression test
(`basic_resume_renders_grace_hopper_sparse_to_expected_text` in
`tests/render_theme.rs`) exercises a JSON Resume document with no
profiles, no project URLs, and no certificate URLs, so a future
re-vendor that breaks any of these guards becomes a committed golden
diff.

## What was NOT patched

The following are deliberately upstream-as-is:

- `resume(...)` show-rule signature (font, paper, margins, accent
  color, author/personal-info alignment, ligatures-disabled text rule).
- `edu`, `work`, `project`, `certificates`, `extracurriculars`
  signatures.
- The `generic-two-by-two`, `generic-one-by-two`, and `dates-helper`
  internals.
- Default font ("New Computer Modern" — bundled in `typst-assets::fonts()`,
  no override needed for reproducibility), default font size (10pt),
  default paper ("us-letter"), default margins (0.5in all sides), and
  default accent color (`#000000`).

The `resume.typ` glue invokes `resume.with(...)` passing only the
contact fields it has data for — CONSTITUTION §5 says a second caller
is the trigger to expose knobs, not the first.

## JSON Resume mapping notes

The ferrocv glue `resume.typ` handles these shape frictions without
touching `basic-resume.typ`:

- **No `profiles` argument:** basic-resume's show rule takes `github`,
  `linkedin`, `personal-site`, and `orcid` as flat string arguments
  (each gets `https://` prepended by `contact-item`). The glue
  case-insensitively matches `basics.profiles[].network` against
  `"GitHub"`, `"LinkedIn"`, etc. and strips any leading `https?://` so
  the helper's prepend doesn't double up. Profiles for networks
  basic-resume doesn't have a slot for (Mastodon, GitLab, etc.) are
  dropped in v1.
- **`personal-site`** comes from JSON Resume `basics.url` with the
  scheme stripped (same prepend rule).
- **`work[]` field renames:** JSON Resume `position` →
  basic-resume `title`; JSON Resume `name` → basic-resume `company`.
- **Sections without a basic-resume helper** — `volunteer`, `awards`,
  `publications`, `skills` — are deferred to a follow-up issue. The
  `basic-resume` template explicitly markets itself as ATS-minimal;
  rendering those sections requires authoring custom layout in the
  glue, and we'd rather track that as scoped work than smuggle it in
  here.
- **`projects` no `role` field:** JSON Resume schema has no per-project
  role; the glue passes `name` only and lets basic-resume's `project`
  helper degrade to a `*name*` heading. `description` and `highlights`
  become bullets.
- **Date ranges** use basic-resume's own `dates-helper` (which renders
  the en-dash with ligatures disabled — the same trick upstream uses
  internally). Open-ended `endDate` becomes `"Present"`.

## Updating

To re-vendor from a newer upstream:

1. Pick the target version on Typst Universe
   (<https://typst.app/universe/package/basic-resume/>); fetch the
   archive directly or pull `src/resume.typ` from the
   `typst/packages` mirror at the new version path.
2. Copy it over `basic-resume.typ` in this directory, overwriting
   byte-for-byte.
3. **Re-apply the §6.1 patch** documented under "Patches applied"
   above. `grep -n "@preview/" basic-resume.typ` must return only the
   ferrocv comment-banner matches; `grep -n "orcid-icon" basic-resume.typ`
   must return no matches at all. If upstream has dropped the
   `@preview/scienceicons` dependency, the grep result will already
   be clean and this step is a no-op.
4. Update the Commit SHA, Commit date, Package version, and Vendor
   date rows in the Provenance table above.
5. Adjust `resume.typ` if any of basic-resume's helper signatures
   changed. `grep -n "^#let " basic-resume.typ` enumerates every top-
   level binding; `grep -n "^#import\|resume\.with\|^\s*\(work\|edu\|project\|certificates\)(" resume.typ`
   enumerates every call site in the glue.
6. Re-verify no new `@preview/...` imports crept in:
   `grep -n "@preview/" basic-resume.typ` must return only the patch-
   banner comments.
7. Re-walk the [shared checklist](../VENDORING_CHECKLIST.md), in
   particular §2 (empty-URL audit — `grep -n "link(" basic-resume.typ`
   and re-verify each guard is still in place).
8. Regenerate goldens: `UPDATE_GOLDEN=1 cargo test --test render_theme basic_resume`
   and inspect the diffs before committing.
