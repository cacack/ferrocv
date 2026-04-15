# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
in this repository.

## Project

`ferrocv` is a Rust CLI that renders [JSON Resume](https://jsonresume.org/)
documents to PDF, HTML, and plain text via [Typst](https://typst.app/),
embedded in-process via the `typst` crate.

**Status: pre-implementation.** See `TODO.md` for the design summary,
phased roadmap, and open questions. `README.md` is the user-facing
entrypoint.

## Principles

See @CONSTITUTION.md for the non-negotiable design principles, non-goals,
and testing doctrine. Treat that file as authoritative; this section
exists only to point at it.

## Conventions

- Conventional commits.
- Rust formatting via `cargo fmt`; lint via `cargo clippy`.
- No committed build artifacts; outputs land in `dist/` (gitignored).

## Related repos

- [`resume`](https://github.com/chrisclonch/resume) — Chris's own
  `resume.json`, currently using the JS-based `resumed` + Playwright
  pipeline. The original design discussion that produced `ferrocv`
  lives in that repo's `NOTES.md`. Once `ferrocv` is usable, the
  `resume` repo will switch to it.
