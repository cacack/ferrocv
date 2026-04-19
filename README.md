# ferrocv

Render [JSON Resume](https://jsonresume.org/) to PDF, HTML, and text via
[Typst](https://typst.app/) — single static binary, no Node or TeX required.

## Status

**Early.** PDF, plain-text, and HTML output all work today (PDF via
any of the registered theme adapters, text and HTML via the native
`text-minimal` theme by default). Additional themes and native-theme
tooling are tracked as
[GitHub issues](https://github.com/cacack/ferrocv/issues) and
organized into phase milestones. HTML uses Typst's upstream-experimental
HTML export — output shape may shift when Typst is bumped; the CLI
surface itself is stable. The non-negotiable design principles live in
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

## Usage

```sh
# Validate a resume against the JSON Resume schema
ferrocv validate resume.json

# Render to PDF using the typst-jsonresume-cv theme
ferrocv render resume.json --theme typst-jsonresume-cv --output resume.pdf

# Render to plain text (defaults to the native `text-minimal` theme;
# `--theme` is optional for text)
ferrocv render resume.json --format text

# Render to HTML (also defaults to `text-minimal`). Note: Typst's HTML
# export is upstream-experimental; output shape may shift across
# ferrocv releases when Typst is bumped.
ferrocv render resume.json --format html
```

The quickest way to try it end-to-end is
[`ferrocv-example`](https://github.com/cacack/ferrocv-example), a
forkable starter template that renders its own `resume.json` to PDF
on every push via GitHub Actions and publishes the result to GitHub
Pages.

`render` defaults to `--format pdf`. `--theme` is required for
`--format pdf` and optional for `--format text` and `--format html`
(both default to `text-minimal`). When `--output` is omitted, the
output lands at `dist/resume.pdf` for PDF, `dist/resume.txt` for text,
and `dist/resume.html` for HTML; parent directories are created as
needed. Both subcommands read from stdin if no path is given.

Exit codes (same for both subcommands):

- `0` — success
- `1` — JSON parsed but failed schema validation (diagnostics on stderr)
- `2` — IO error, JSON parse error, unknown theme/format, or render error

No network is touched — the schema, theme, and fonts are all compiled
into the binary.

## Development

Run the full CI check suite locally before pushing:

```sh
make preflight
```

This mirrors `.github/workflows/ci.yml` and runs `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test`, `cargo-deny`, `cargo-audit`,
and `typos`. Individual checks are available as their own targets
(`make clippy`, `make test`, ...); run `make help` for the full list.

First-time setup installs the non-cargo-stock tools:

```sh
make install-tools
```

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
