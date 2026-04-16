# ferrocv

Render [JSON Resume](https://jsonresume.org/) to PDF, HTML, and text via
[Typst](https://typst.app/) — single static binary, no Node or TeX required.

## Status

**Pre-implementation.** Work is tracked as [GitHub
issues](https://github.com/cacack/ferrocv/issues), organized into phase
milestones. Tracking issues stand in for future phases until their scope
is activated. The non-negotiable design principles live in
[`CONSTITUTION.md`](./CONSTITUTION.md).

## Why

The JSON Resume schema is a sound single-source-of-truth for resume data,
but its JavaScript theme ecosystem is thin and fragile (many themes are
abandoned, others ship with broken dependencies). This project keeps the
schema and replaces the rendering pipeline with something more robust:

- **Rust** for a single-binary CLI with no runtime dependencies.
- **Typst** for modern typesetting — embeddable as a crate, no TeX distro
  needed, with a growing ecosystem of resume templates.
- **JSON Resume v1.0.0** remains the canonical input format.

## Goals

- Validate `resume.json` against the JSON Resume schema.
- Compile to PDF in-process via the `typst` crate (no subprocess).
- Emit HTML and plain text as first-class outputs, not afterthoughts.
- Ship adapters over popular Typst Universe templates so users have
  visual variety from day one.
- Define a native theme contract so new themes can target JSON Resume
  directly.

## Non-goals

- Replacing the JSON Resume schema or project.
- Supporting arbitrary input formats (Markdown, YAML, etc.).
- Becoming a general-purpose Typst build tool.

## Prior art

- [`fruggiero/typst-jsonresume-cv`](https://github.com/fruggiero/typst-jsonresume-cv)
  — Typst template that accepts JSON Resume data.
- [`fantastic-cv`](https://typst.app/universe/package/fantastic-cv/)
  — Typst Universe template with a JSON Resume-shaped API.
- [`basic-resume`](https://typst.app/universe/package/basic-resume/)
  and [`modern-cv`](https://typst.app/universe/package/modern-cv/)
  — Typst Universe resume templates we plan to ship adapters for.
- [`jsonresume-renderer`](https://lib.rs/crates/jsonresume-renderer)
  — Rust CLI that renders JSON Resume via Tera templates (not Typst).
- [`typst.ts`](https://github.com/Myriad-Dreamin/typst.ts) — proves Typst
  is embeddable outside its native CLI.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](./LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([`LICENSE-MIT`](./LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option. This is the standard Rust ecosystem dual license.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms
or conditions.
