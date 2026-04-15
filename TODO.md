# TODO

Working notes, roadmap, and open questions for `ferrocv`. This is the
handoff document from the design discussion that happened in my personal resume repo.

## Design summary

**Diagnosis.** JSON Resume's *schema* is sound; the *JS theme ecosystem*
is the weak link (abandoned themes, broken deps). Keep the schema,
replace the renderer.

**Direction: Typst over LaTeX/HTML-CSS.**
- LaTeX: gold-standard typography, but huge runtime dep (TeX distro)
  and hostile ergonomics for theme authors.
- HTML/CSS (current state of the resume repo): weakest for complex
  layouts under paged output; Playwright is a heavy subprocess.
- **Typst**: Rust-native (embeddable as a crate — no subprocess, single
  static binary), modern LaTeX replacement, growing resume template
  ecosystem, typography close enough to LaTeX for 1–2 page docs.

**Proposed flow.**
- `resume.json` (JSON Resume v1.0.0 schema) is the canonical input.
- `ferrocv` Rust binary that:
  - Validates the input against the schema (`jsonschema` crate).
  - Resolves a theme (local path or Typst Universe package).
  - Compiles via the `typst` crate in-process — no subprocess, no TeX.
  - Emits PDF directly; HTML via Typst's HTML export; plain text via
    a built-in minimal "plaintext" theme.

**CLI shape (proposed).**
```
ferrocv render --theme <name> --format pdf,html,txt [--output dist/]
ferrocv validate [resume.json]
ferrocv new-theme <name>
```

**Theme strategy.**
- **Adapter layer (launch):** thin Typst wrappers over 3–5 popular
  Universe templates that map JSON Resume fields to each template's
  expected parameters. Per-theme cost is tens-to-low-hundreds of Typst
  LOC. Breaks if upstream bumps API — acceptable maintenance.
- **Native theme contract (grow into):** a theme is a Typst module
  that imports `resume.json` natively (`json()` is built-in) and
  exports `render(data)` returning content. Mirrors the JSON Resume
  `render(resume)` convention.

**Coupling model.** Schema ↔ adapter is the real coupling. Adapter ↔
template is loose. Themes are constrained to "what JSON Resume can
express"; for a resume that's rarely limiting. The `x-` extension
prefix handles anything novel.

## Phases

### Phase 0 — Project scaffolding
- [x] Pick license — dual MIT/Apache-2.0 (Rust convention).
- [x] CI baseline: GitHub Actions with SHA-pinned actions running
      `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
      `cargo-deny`, and `cargo-audit`. Jobs guard on `Cargo.toml`
      existing so they go green until `cargo init` lands.
- [x] Release pipeline: `release-please` (conventional commits) opens
      release PRs on `main`; tagged releases trigger cross-platform
      builds for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`,
      and `x86_64-pc-windows-msvc` with SHA-256 checksums.
- [x] Dependabot for `github-actions` and `cargo`.
- [x] Decide repo layout: **single crate** with `src/main.rs` +
      `src/lib.rs`. Revisit (promote to workspace) if a second
      consumer of the library emerges or we split out
      `ferrocv-themes-*`.
- [x] `cargo init` with binary + library split in a single `ferrocv`
      crate. `release-please-config.json` flipped from
      `release-type: simple` to `release-type: rust` so version bumps
      target `Cargo.toml`.
- [ ] Reserve names: register placeholders on
      [crates.io](https://crates.io/) and
      [npm](https://www.npmjs.com/) once there's enough to publish.

### Phase 1 — MVP: validate + render PDF
- [ ] JSON Resume v1.0.0 schema bundled in-binary
      (`include_str!`/`include_bytes!`)
- [ ] `ferrocv validate` — schema validation with clear errors
- [ ] Embed Typst via the `typst` crate; compile to PDF in-process
- [ ] Wire the first theme adapter over
      [`fruggiero/typst-jsonresume-cv`](https://github.com/fruggiero/typst-jsonresume-cv)
      (it already accepts JSON Resume directly)
- [ ] `ferrocv render --format pdf` end-to-end on a real `resume.json`

### Phase 2 — Multi-format output
- [ ] HTML output via Typst's experimental HTML export. Re-confirm
      maturity at the time we get here.
- [ ] Plain text output via a built-in minimal Typst theme (or direct
      from JSON if Typst's text export is weak)
- [ ] Decide DOCX strategy: drop, or HTML→pandoc fallback

### Phase 3 — Theme adapters
- [ ] Adapter for [`basic-resume`](https://typst.app/universe/package/basic-resume/)
- [ ] Adapter for [`modern-cv`](https://typst.app/universe/package/modern-cv/)
- [ ] Adapter for [`fantastic-cv`](https://typst.app/universe/package/fantastic-cv/)
      (already JSON Resume-shaped)
- [ ] Document the adapter pattern so contributors can add more
- [ ] Theme resolution: local path, bundled, Typst Universe package

### Phase 4 — Native theme contract
- [ ] Specify the contract: `render(data) -> content`, `data` is the
      parsed JSON Resume document with extensions surfaced
- [ ] `ferrocv new-theme <name>` scaffold
- [ ] Build 1–2 reference native themes (one minimal, one richer)

### Phase 5 — Filtering and audience-aware rendering
Pulled forward from the resume repo's design notes. The renderer
should support producing different outputs from the same source:
- [ ] `--since YYYY` to drop older roles
- [ ] `--max-bullets N` per role
- [ ] `--include-salary` / `--redact pii` toggles
- [ ] Multiple themes per audience (govt application vs. standard
      resume vs. one-pager)

## Open questions

- [ ] Confirm Typst HTML export maturity at the time of implementation.
- [ ] DOCX strategy — drop entirely, or keep an HTML→pandoc fallback?
- [ ] How should renderer filter flags interact with themes — preprocess
      JSON in Rust before handing to the theme, or pass as theme
      parameters? Probably preprocess (keeps themes simple), but worth
      sanity-checking.
- [ ] Library vs. binary split — is there demand for `ferrocv-core` as
      a crate others can embed, or is the CLI sufficient?
- [ ] Theme distribution: do we host adapters in this repo, or split
      them into per-theme repos under a `ferrocv-themes-*` namespace?
- [ ] How to handle JSON Resume `x-` extension fields cleanly across
      adapters that don't know about them (silent drop vs. warn)?

## References

- [JSON Resume schema v1.0.0](https://github.com/jsonresume/resume-schema)
- [JSON Resume project](https://jsonresume.org/)
- [Typst documentation](https://typst.app/docs/)
- [Typst `json()` data loader](https://typst.app/docs/reference/data-loading/json/)
- [`typst` crate on crates.io](https://crates.io/crates/typst)
- [Typst Universe (templates/packages)](https://typst.app/universe/)
- [`fruggiero/typst-jsonresume-cv`](https://github.com/fruggiero/typst-jsonresume-cv)
- [`fantastic-cv`](https://typst.app/universe/package/fantastic-cv/)
- [`basic-resume`](https://typst.app/universe/package/basic-resume/)
- [`modern-cv`](https://typst.app/universe/package/modern-cv/)
- [`jsonresume-renderer` (Tera-based, adjacent prior art)](https://lib.rs/crates/jsonresume-renderer)
- [`typst.ts` / `reflexo-typst`](https://github.com/Myriad-Dreamin/typst.ts)
