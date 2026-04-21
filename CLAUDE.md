# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
in this repository.

## Project

`ferrocv` is a Rust CLI that renders [JSON Resume](https://jsonresume.org/)
documents to PDF, HTML, and plain text via [Typst](https://typst.app/),
embedded in-process via the `typst` crate.

PDF, plain-text, and HTML rendering all work today (released as v0.3.0).
The phased roadmap now lives in [GitHub
issues](https://github.com/cacack/ferrocv/issues); `TODO.md` holds only
unresolved design questions. `README.md` is the user-facing entrypoint.

## Commands

`Makefile` is the source of truth for CI-parity checks — command
strings there are kept in lockstep with `.github/workflows/ci.yml`.

- `make preflight` — full CI check suite (fmt-check, clippy, test,
  deny, audit, typos). Run before pushing.
- `make test` / `make clippy` / `make fmt` — individual targets.
- `make install-tools` — first-time install of the non-stock tools
  (`cargo-deny`, `cargo-audit`, `typos-cli`).
- `make fuzz` / `make fuzz-parse` / `make fuzz-validate` —
  local cargo-fuzz smoke (60s per target, nightly Rust required).
  Nightly CI runs these for 120s each; see
  `.github/workflows/fuzz.yml`.
- A single test: `cargo test --test <file> <name>` (integration tests
  live under `tests/`, e.g. `cargo test --test render_cli`).

## Architecture

- **Embedded Typst (CONSTITUTION §2).** Rendering is
  `typst::compile(&world)` in-process, then `typst-pdf`, `typst-html`,
  or a frame-walk text extractor. No subprocess, no shell-out, ever.
  `src/render.rs` owns the `FerrocvWorld` and the three exporters;
  `FerrocvWorld` refuses `@preview/...` package imports to uphold §6.1
  (no network at render time).
- **Theme registry (`src/theme.rs`).** A static `&[&Theme]` slice of
  Typst source bundles `include_bytes!`'d at compile time. Two kinds
  live here per CONSTITUTION §4:
  - *Adapters* wrap upstream Typst Universe templates vendored under
    `assets/themes/<name>/` — each with its own `VENDORING.md` recording
    upstream SHA and any patches (e.g. `modern-cv` was patched to drop
    `@preview/fontawesome` and `@preview/linguify`).
  - *Native themes* (currently just `text-minimal`) author Typst
    directly against the JSON Resume schema; this is what
    `--format text` and `--format html` default to.
- **Schema embedding.** `assets/schema/jsonresume-v1.0.0.json` is
  vendored and baked into the binary via `include_str!` in `src/lib.rs`.
  `jsonschema` is built without default features to avoid pulling
  `reqwest` (§6.1).
- **Tests.** Scenario tests spawn the built binary via `assert_cmd`
  (`tests/validate_cli.rs`, `tests/render_cli.rs`). Theme regressions
  are caught by golden text files in `tests/goldens/` — the fixtures
  `render_full.json` and `render_sparse.json` both get rendered through
  every theme and diffed against committed text extractions.

## Principles

See @CONSTITUTION.md for the non-negotiable design principles, non-goals,
and testing doctrine. Treat that file as authoritative; this section
exists only to point at it.

## Conventions

- **Commits**: Conventional Commits, restricted to these 7 types
  (anything else is rejected by release-please's `changelog-sections`):
  - In changelog: `feat`, `fix`, `perf`
  - Hidden: `docs`, `refactor`, `ci`, `chore`
  - `feat` and `fix` are reserved for user-facing changes; tooling and
    build changes are `ci` or `chore`.
- **PR titles**: descriptive prose, **not** Conventional Commits format.
  release-please reads conventional types from the commit log; mirroring
  them in PR titles produces duplicate changelog entries on squash merge.
- **Branches**: `feat/<topic>`, `fix/<topic>`, `ci/<topic>`, etc. Add a
  `NNN-` prefix once we have a GitHub issue number to anchor to.
- Rust formatting via `cargo fmt`; lint via `cargo clippy`.
- No committed build artifacts; outputs land in `dist/` (gitignored).

## Related repos

- [`resume`](https://github.com/chrisclonch/resume) — Chris's own
  `resume.json`, currently using the JS-based `resumed` + Playwright
  pipeline. The original design discussion that produced `ferrocv`
  lives in that repo's `NOTES.md`. Once `ferrocv` is usable, the
  `resume` repo will switch to it.
