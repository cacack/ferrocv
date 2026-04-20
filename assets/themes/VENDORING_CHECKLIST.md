# Vendoring checklist

Runbook for vendoring a new theme adapter, or re-vendoring an existing
one after an upstream or Typst bump. Each per-theme `VENDORING.md` links
back here; this file codifies the hygiene steps we keep rediscovering
the hard way.

Scope: **adapters only** (themes under `assets/themes/` that wrap an
upstream Typst Universe template). Native themes like `text-minimal`
author Typst directly and do not need this checklist.

Work top-to-bottom. Each item is a two-minute check, not a policy
debate — if the grep comes back clean, tick the box and move on.

## 1. No `@preview/...` imports

Upstream Typst Universe templates routinely import other Typst Universe
packages (`fontawesome`, `linguify`, `scienceicons`, etc.). The
`FerrocvWorld` rejects any `FileId` carrying a `PackageSpec` at render
time — CONSTITUTION §6.1 forbids network fetches — so **any theme that
imports an `@preview/...` package is dead on arrival**.

```sh
grep -n "@preview/" assets/themes/<theme>/*.typ
```

- [ ] Returns no matches.
- [ ] If matches exist: remove the import *and* rewrite every call site.
      Record each one in the theme's `VENDORING.md` under "Patches
      applied" with a before/after snippet. See
      [`modern-cv/VENDORING.md`](./modern-cv/VENDORING.md) Patches A and
      B for worked examples (fontawesome + linguify).

## 2. Empty-URL safety audit

Typst `link("")` is a fatal error in 0.14.x (`error: URL must not be
empty`). JSON Resume v1.0.0 makes every `url` field optional, so
real-world resumes trigger this constantly. The fantastic-cv empty-URL
bug ([#59]) was a full regression on a Typst bump because we had not
audited this proactively.

[#59]: https://github.com/cacack/ferrocv/issues/59

```sh
grep -n "link(" assets/themes/<theme>/*.typ
```

- [ ] For every call site, trace the argument back to its source. If it
      can ever be an empty string, the call site must be guarded.
- [ ] Acceptable guard patterns (pick whichever is closest to upstream
      — **do not normalize across themes**, each theme's pattern
      minimizes re-vendoring diff):
  - **Inline ternary** (fantastic-cv): `if url.len() == 0 { text } else
    { link(url)[text] }`.
  - **Presence check** (modern-cv): `if ("email" in author) [ ...
    link(...) ]`.
  - **Empty-string guard in a helper** (typst-jsonresume-cv): wrap the
    `link()` inside `if value != "" { ... }`.
- [ ] Record the audit outcome in the theme's `VENDORING.md` under
      "Empty-URL safety audit" — call sites, guard pattern, and the
      regression test that exercises the sparse path
      (`render_accepts_sparse_schema_valid_resume` in
      `tests/render_cli.rs`).

## 3. License consistency

Upstream repos sometimes disagree with themselves: the `LICENSE` file
says one thing, `typst.toml`'s `license` field says another. Vendoring
the `LICENSE` file verbatim is correct, but the discrepancy must be
noted so a future re-vendorer doesn't silently "fix" it.

- [ ] Compare upstream `LICENSE` and `typst.toml`'s `license` field.
- [ ] Vendor the actual `LICENSE` file's text into
      `assets/themes/<theme>/LICENSE`.
- [ ] If the two disagree, record it in the "License" section of the
      theme's `VENDORING.md`. See
      [`fantastic-cv/VENDORING.md`](./fantastic-cv/VENDORING.md) lines
      28-53 for a worked example (MIT in `typst.toml`, Unlicense in
      `LICENSE`).
- [ ] Verify the upstream license is compatible with ferrocv's
      MIT-OR-Apache-2.0 dual license.

## 4. Regenerate goldens and inspect visually

Any vendoring change — initial vendor, re-vendor, Typst bump, or a
patch re-application — can shift pixels. The golden text extractions
catch that.

```sh
UPDATE_GOLDEN=1 cargo test --test render_theme
```

- [ ] Run the command above, then `git diff tests/goldens/` and read
      every line of the diff. Semantically sensible changes get
      committed; anything unexpected is a bug.
- [ ] Render at least one PDF manually and open it. The golden diffs
      cover text content but not layout; a broken layout shows up
      visually long before it breaks a test.
- [ ] `make preflight` must be green after the update.

## Final sanity pass

- [ ] The per-theme `VENDORING.md` provenance table (upstream URL,
      commit SHA, commit date, package version, vendor date) is
      current.
- [ ] The theme's `VENDORING.md` links back to this checklist in its
      "See also" line.
- [ ] `make preflight` is green.
