# Strategic Snapshot — 2026-10-03

## Repo metadata
- Root: /Users/chris/devel/home/ferrocv
- Branch: main
- HEAD: a2a909d
- Origin: git@github.com:cacack/ferrocv.git
- Generated: 2026-10-03T13:17:59Z

## CONSTITUTION.md (scoring rubric)

# Constitution

The non-negotiables for `ferrocv`. This document answers **what** we're
building and **why**. *How* lives in the code and [GitHub
issues](https://github.com/cacack/ferrocv/issues).

Amendments require a PR that updates this file and explains the
reasoning in the commit message. Everything else (style, structure,
tooling choices) is open to iteration.

## Core principles

### 1. JSON Resume is the canonical input

We consume [JSON Resume v1.0.0](https://github.com/jsonresume/resume-schema)
unmodified. We do not invent a competing schema, a superset, or a
"friendlier" dialect.

- **Why:** the schema is the durable asset of the JSON Resume project;
  the renderer ecosystem is the weak link. Replacing the renderer only
  works if users can bring their existing `resume.json` as-is.
- **Extension mechanism:** the `x-` prefix. Anything not expressible in
  stock JSON Resume goes under `x-<namespace>` fields that themes may
  opt into. No other extension points.

### 2. Embed Typst; never subprocess it

Typst is consumed as the `typst` Rust crate, compiled in-process. We
do not shell out to the `typst` CLI, Node, Playwright, a browser, or a
TeX distribution at runtime.

- **Why:** the entire reason this tool exists is to replace a
  multi-runtime pipeline (Node + Playwright, or LaTeX) with a single
  static binary. A subprocess invalidates the premise.
- **Applies to:** release binaries, tests, examples. Build-time tooling
  (CI, dev scripts) is exempt.

### 3. Multi-format output is first-class

PDF, HTML, and plain text are parallel targets. None is a second-class
fallback; none may be permanently gated behind a feature flag.

- **Why:** resumes get consumed by ATS systems, web profiles, and
  email bodies — not just printed. Designing for PDF-only bakes in
  assumptions that are painful to undo later.
- **Implication:** the theme contract and the core data model must not
  encode PDF-specific assumptions (page breaks, fixed fonts, absolute
  positioning) in ways that cannot degrade to HTML and text.

### 4. Two theme interfaces, kept separable

- **Adapters** wrap upstream Typst Universe templates by mapping JSON
  Resume fields into the template's parameters. Breakage on upstream
  changes is accepted.
- **Native themes** implement a `render(data) -> content` contract
  directly against parsed JSON Resume data.

These are distinct layers. Adapter code does not leak into native
themes; native themes do not depend on adapter internals.

- **Why:** adapters give us visual variety on day one; native themes
  give us a durable, JSON-Resume-shaped contract long-term. Conflating
  them produces a leaky abstraction that serves neither well.

### 5. Simple now; iterate later

Phase 1 is built for Phase 1. We do not pre-engineer extension points,
plugin systems, or configuration surfaces for phases we have not
started.

- **Why:** this is a personal tool graduating to open-source; the
  cost of YAGNI here is low and the cost of premature abstraction is
  high.
- **In practice:** when in doubt, pick the narrower, more specific
  solution. A second caller is the trigger to generalize, not the
  first.

### 6. Trust is a feature, not a footnote

A resume is concentrated PII — name, address, phone, email, employer
history. Users must be able to run this tool and know exactly where
their data goes. The following are hard commitments; weakening any of
them requires a constitutional amendment, not a feature PR.

- **No network calls in `render` or `validate`, full stop.**
  `ferrocv render` and `ferrocv validate` are fully offline. Themes
  ship vendored in-tree (`assets/themes/`) and are baked into the
  binary; the JSON Resume schema is vendored the same way. The
  embedded Typst `World` never fetches a `@preview/...` package; on
  cache miss it rejects the import with a structured "package not
  found" diagnostic. Rendering may read from the local installer cache
  populated by a prior `ferrocv themes install` (see next bullet); that
  is a local filesystem read, not a network call, and does not weaken
  the `render`-is-offline guarantee. The cache-read allowance covers
  both the primary spec resolved at the CLI boundary and any
  `@preview/...` imports the Typst World resolves on a cached
  package's behalf at compile time — a same-class extension of the
  same local-filesystem-read principle, gated behind the same
  `install` Cargo feature. Default-features builds do not include the
  cache reader at all and keep the historical blanket rejection of
  every `@preview/...` import.
- **`ferrocv themes install` is the single, enumerated network-permitted
  entry point.** It is an explicit, user-initiated subcommand that
  fetches only from the Typst Universe `@preview` registry over HTTPS
  (`https://packages.typst.org/preview/<name>-<version>.tar.gz`); it
  is never invoked transitively from `render` or `validate`; its
  network-capable dependencies live behind a Cargo feature flag
  (`install`) so the default build contains no network code at all.
  Fetches initiated by `themes install` recursively include the
  transitive `@preview/...` packages reachable from the requested
  package's source — installing `@preview/foo:1.0` whose source imports
  `@preview/bar:2.0` populates both cache entries in one invocation.
  This is a same-class extension of the existing network surface, not
  a new one: the registry, the protocol, the integrity model, and the
  `install` Cargo feature gate are unchanged.
  Package integrity is established by TLS only: `ferrocv` does not
  verify upstream checksums or signatures for v1 because the Typst
  Universe registry does not publish them. Users who need stronger
  integrity guarantees can vendor the theme manually under
  `assets/themes/`. Any additional network-touching operation — a
  different registry, a signature verifier reaching out to a key
  server, a theme search index — requires a further constitutional
  amendment, not a feature PR.
- **No telemetry, ever.** No usage pings, no crash reports, no opt-in
  "help us improve" toggle, no analytics SDK. Not now, not later.
- **Resume data never leaves the process.** We read `resume.json`, we
  write files to disk the user specified. That is the entire
  data-flow surface. No uploads, no cloud rendering, no LLM calls, no
  "share" features.
- **Themes run under Typst's native sandbox, nothing more.** We do
  not extend the Typst runtime with filesystem-wide, network, or
  shell-escape capabilities to make theme authoring "easier." If
  Typst doesn't grant a capability, neither do we.
- **Reproducible, verifiable releases.** Tagged releases are built
  from tagged source in CI; release artifacts are checksummed, and
  (when tooling allows) signed. *Aspirational for Phase 0+:* state
  it now so it's on the record and gets wired up as soon as there's
  something to release.

**Why:** the pitch is "single static binary, no Node, no TeX." That
pitch is also the security story — fewer moving parts, less
attack surface, nothing phoning home. Making trust explicit keeps us
from quietly trading it away for convenience later.

### 7. Targeted projection from a single master

The point of `ferrocv` is not only to render a resume — it is to let a
user maintain **one comprehensive master `resume.json`** (every role,
every highlight; 5+ pages printed in full) and emit **targeted,
audience-specific cuts** from it: a focused two-page document that is a
*subset/projection* of the master, never a separately maintained file.

- **Projection is a distinct stage upstream of rendering.** It takes
  the master document plus a selection spec and produces a **derived
  document that is itself still valid JSON Resume**, which then flows
  into the existing render pipeline unchanged. The master is consumed
  unmodified (§1); projection is an additive transform that emits a
  valid JSON Resume subset, not a schema fork.
- **Two layers of selection.** *Mechanical* — drop roles before a date,
  cap highlights per role, redact PII. *Curated* — select content by
  audience tags carried under `x-` fields (§1's extension mechanism),
  so "the security cut" keeps the highlights tagged for it rather than
  the first N by position. Curated selection is the headline; mechanical
  selection is the cheap complement.
- **Selection lives in Rust, never in themes.** The projection stage
  preprocesses the document; themes receive an already-narrowed valid
  JSON Resume and stay ignorant of audiences and filters. This keeps
  the theme contract simple (§5) and the layers separable (§4).
- **Projection selects and omits; it never rewrites or generates.** We
  do not reword bullets, summarize, or invent content — that would make
  this an authoring tool and require the LLM calls §6 rules out. The
  user writes the master; `ferrocv` chooses what to show.

- **Why:** plenty of tools render a JSON Resume to PDF — that capability
  is a commodity and not, by itself, a reason for `ferrocv` to exist.
  Maintaining a single source of truth and generating tailored cuts per
  application is the differentiating value. A data model built around
  "one document in, one document out" would quietly foreclose it, so we
  state it as a principle rather than discover it as a retrofit.

## Non-goals

These are deliberately out of scope. Proposals to add them belong in an
amendment PR, not a feature PR.

- Replacing, forking, or extending the JSON Resume schema.
- Supporting input formats other than JSON Resume (Markdown, YAML,
  HR-XML, etc.).
- Becoming a general-purpose Typst build tool.
- Shipping a hosted service, web UI, or SaaS wrapper.
- Authoring or rewriting resume content. Projection (§7) selects and
  omits from a master the user wrote; it does not generate, summarize,
  or reword bullets, and it does not auto-fit content to a page count.

## Testing doctrine

TDD/BDD in spirit, not ceremony. The rules are narrow and enforceable:

1. **Every CLI-visible behavior has a scenario-style test.** Inputs,
   flags, and observable output (stdout, exit code, generated file
   existence). These read as "given `resume.json` and `--theme X`,
   when I run `render`, then `dist/resume.pdf` exists and is a valid
   PDF." Write these before the implementation of the behavior.
2. **Every theme (adapter or native) has a golden-file test.** A
   committed reference output (PDF bytes are fragile; prefer a
   deterministic intermediate — Typst source, HTML, or a normalized
   text extraction) that regressions must explain.
3. **Schema validation has negative tests.** For each class of
   invalid input we claim to catch, a test asserts we catch it with a
   useful error.
4. **No mocking Typst.** Compilation tests run the real embedded
   Typst. If that's too slow, fix the slowness, don't fake the
   compiler.

Unit tests beyond these are welcome but not mandated. Coverage
percentages are not a goal.

## Amendments

- Update this file.
- In the commit message, state what changed and why.
- If a principle is being weakened or removed, call that out
  explicitly — silent softening is how constitutions rot.

## Success Criteria scorecard (measured before personas)

# Success Criteria Scorecard — 2026-10-03

Evidence for the personas — no findings, no verdict.

No Success Criteria in CONSTITUTION.md — nothing to score.

## README excerpt

# ferrocv

[![CI](https://github.com/cacack/ferrocv/actions/workflows/ci.yml/badge.svg)](https://github.com/cacack/ferrocv/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/cacack/ferrocv/branch/main/graph/badge.svg)](https://codecov.io/gh/cacack/ferrocv)
[![CodeQL](https://github.com/cacack/ferrocv/actions/workflows/codeql.yml/badge.svg)](https://github.com/cacack/ferrocv/actions/workflows/codeql.yml)
[![crates.io](https://img.shields.io/crates/v/ferrocv.svg)](https://crates.io/crates/ferrocv)
[![docs.rs](https://img.shields.io/docsrs/ferrocv)](https://docs.rs/ferrocv)
[![Downloads](https://img.shields.io/crates/d/ferrocv)](https://crates.io/crates/ferrocv)
[![License](https://img.shields.io/crates/l/ferrocv)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.92-orange)](Cargo.toml)
[![Dependencies](https://deps.rs/repo/github/cacack/ferrocv/status.svg)](https://deps.rs/repo/github/cacack/ferrocv)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/cacack/ferrocv/badge)](https://scorecard.dev/viewer/?uri=github.com/cacack/ferrocv)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg)](https://www.conventionalcommits.org)

Render [JSON Resume](https://jsonresume.org/) to PDF, HTML, and text via
[Typst](https://typst.app/) — single static binary, no Node or TeX required.

## Status

**Early.** PDF, plain-text, and HTML output all work today (PDF via
the native PDF-first `classic` default or any registered theme, plain
text via the native `text-minimal` default, and HTML via the native
`html-minimal` semantic theme). Additional themes and native-theme
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

# Render to PDF (defaults to the native PDF-first `classic` theme;
# `--theme` is optional)
ferrocv render resume.json

# Keep the previous extraction-tuned PDF default (pre-v0.9 `text-minimal`)
ferrocv render resume.json --theme text-minimal --output resume.pdf

# Or use an adapter that wraps an upstream Typst Universe template
ferrocv render resume.json --theme typst-jsonresume-cv --output resume.pdf

# `classic` shows a "Tailored for: <label>" tagline when the resume's
# `meta` object carries an `x-audience` string, e.g.
#   { "meta": { "x-audience": "security" }, ... }
# (the tag lives under `meta`, not the document root, because JSON Resume
# permits `x-` extension fields inside objects but not at the top level)

# Render to plain text (defaults to `text-minimal`)
ferrocv render resume.json --format text

# Render to HTML (defaults to the native `html-minimal` semantic theme).
# Note: Typst's HTML export is upstream-experimental; output shape may
# shift across ferrocv releases when Typst is bumped.
ferrocv render resume.json --format html

# List bundled themes (machine-readable, one name per line)
ferrocv themes list

# Scaffold a starter native theme to author your own
ferrocv themes new mytheme

# Project a master resume into a narrower cut (mechanical filters):
# drop roles that ended before 2015, cap each job at 4 bullets, and
# strip PII. Writes the derived JSON Resume to stdout (or use -o).
ferrocv tailor master.json --since 2015 --max-bullets 4 --redact pii -o cut.json

# Same filters straight to a rendered PDF — no intermediate file
ferrocv render master.json --since 2015 --max-bullets 4 --redact pii --theme typst-jsonresume-cv
```

The quickest way to try it end-to-end is
[`ferrocv-example`](https://github.com/cacack/ferrocv-example), a
forkable starter template that renders its own `resume.json` to PDF
on every push via GitHub Actions (using the `setup-ferrocv` composite
action below) and publishes the result to GitHub Pages.

`render` defaults to `--format pdf`. `--theme` is optional for every
format: PDF defaults to the native PDF-first `classic` theme, text to
the native `text-minimal` theme, and HTML to the native `html-minimal`
semantic theme. (The PDF default became `classic` in v0.9; if you relied
on the older extraction-tuned PDF, pass `--theme text-minimal`.) When
`--output` is omitted, the output lands at `dist/resume.pdf` for PDF,
`dist/resume.txt` for text, and `dist/resume.html` for HTML; parent
directories are created as needed. `validate`, `render`, and `tailor`
read from stdin if no path is given.

### Projection: one master, many cuts

The point of ferrocv is to maintain **one comprehensive master
`resume.json`** and emit **targeted, audience-specific cuts** from it
(CONSTITUTION §7). Projection is a stage *upstream* of rendering: it
reads the master unmodified and produces a derived document that is
itself valid JSON Resume, which flows into the normal render pipeline.

For a full walkthrough — tagging a master, the include-by-default rule,
worked cuts, and the gotchas — see the [tailoring
guide](docs/tailoring.md); [`examples/master.resume.json`](examples/master.resume.json)
is a tagged master to start from. The rest of this section is the
condensed reference.

`ferrocv tailor` runs that stage and stops, emitting the derived
document so you can inspect, commit, or pipe it. The same flags are
also available on `render`, which projects then renders in one shot —
so `ferrocv render master.json --since 2015` is equivalent to
`ferrocv tailor master.json --since 2015 | ferrocv render`.

The **curated** filter (the headline — keep what's *relevant*, not the
first N by position):

- `--audience <name>` — keep only content tagged for that audience. Tag
  an array entry (a `work`/`volunteer`/`project`/… object) by adding an
  `x-ferrocv` object beside its fields: `"x-ferrocv": { "audience":
  ["security"] }`. Tag individual bullets with an index-parallel
  `"x-ferrocv": { "highlights": [["security"], [], ["leadership"]] }`,
  one tag-list per `highlights` entry. **Untagged content (or an empty
  `[]` tag) is universal** — kept in every cut — so you can adopt tags
  incrementally; only content tagged for *other* audiences is dropped.
  The consumed `x-ferrocv` metadata is stripped from the derived
  document. (Schema and rationale: ADR 0004.)

The **mechanical** filters (theme-agnostic):

- `--since <YYYY|YYYY-MM|YYYY-MM-DD>` — drop `work` entries that ended
  before the cutoff. Ongoing roles (no `endDate`) are always kept.
- `--max-bullets <N>` — cap every `highlights` list at the first N
  bullets. Runs *after* `--audience`, so it caps the already-curated
  set.
- `--redact pii` — remove `basics.location`, `basics.phone`, and
  `basics.email` from the cut. Identity fields (`name`, `label`,
  `summary`, `url`, `profiles`) are kept.

With no projection flags, `render` behaves exactly as it does on any
input — projection is opt-in and inert by default.

`tailor` writes the derived document to `--output <file>`, or to stdout
when `--output` is omitted, so it composes in a pipe; all diagnostics
go to stderr. Note that stdout-by-default prints the full resume,
including any PII not removed by `--redact` — prefer `--output <file>`
for unattended or shared/recorded contexts.

`themes list` prints registered theme names to stdout, one per line,
sorted lexicographically, with no decoration — a stable
machine-readable contract.

`themes new <name>` scaffolds a starter native theme to author your own.
It writes a new `<name>/` directory (in the current directory, or under
`--out <dir>`) containing a ready-to-edit `resume.typ` and a `golden.txt`
test stub. The emitted `resume.typ` `#import`s ferrocv's shared
native-theme prelude and renders the major JSON Resume sections, so it
renders straight away — point `render --theme` at the file:

```sh
ferrocv themes new mytheme
ferrocv render resume.json --theme mytheme/resume.typ --output resume.pdf
```

`<name>` must be a bare directory name (letters, digits, `-`, `_`; no
leading `-`, no path separators or `..`), and the command refuses to
write into an existing target rather than clobber it. For the full
walkthrough — the `render(data) -> content` contract, the prelude API,
and golden-test setup — see
[`docs/native-themes.md`](docs/native-themes.md); the bundled `classic`
theme is the worked example.

`themes install` resolves transitive `@preview/...` dependencies
recursively: installing one package also fetches every `@preview/...`
package its source declares as an import, hydrating the local cache for
the whole graph in one invocation instead of N. Cycles in declared
imports are detected and do not loop; missing transitive packages
hard-fail with the primary still cached for retry. The primary's cache
path is printed to stdout (one line, scriptable); a human-readable
summary of any transitive deps newly installed or already cached is
printed to stderr, e.g.

## Project metadata
```
[package]
name = "ferrocv"
version = "0.9.0"
edition = "2024"
rust-version = "1.92"
description = "Render JSON Resume documents to PDF, HTML, and plain text via embedded Typst."
license = "MIT OR Apache-2.0"
repository = "https://github.com/cacack/ferrocv"
readme = "README.md"
keywords = ["resume", "cv", "json-resume", "typst", "pdf"]
categories = ["command-line-utilities"]
include = [
    "src/**/*",
    "assets/schema/**/*",
    "assets/themes/**/*",
    "Cargo.toml",
    "README.md",
    "LICENSE-*",
]

[lib]
```

## Repository label vocabulary
bug, documentation, duplicate, enhancement, good first issue, help wanted, invalid, question, wontfix, chore, tracking, area:ci, area:rendering, area:themes, area:cli, area:schema, value:low, value:high, value:medium, effort:low, effort:medium, effort:high, autorelease: pending, autorelease: tagged, dependencies, github_actions, rust, fuzz-failure, priority:high, priority:medium, priority:low, feature:projection

## Open issues
<untrusted-issue-data>
| # | Title | Labels | Milestone |
|---|---|---|---|
| #230 | Auto-merge workflow fails instead of skipping when there is nothing to do | area:ci, value:medium, effort:low, priority:medium | - |
| #229 | Adopt shared reusable dependabot-automerge workflow | area:ci, value:medium, effort:medium, priority:medium | - |
| #196 | Projection: collapse old work entries to one-liners instead of dropping them | enhancement, area:cli, value:medium, effort:medium, priority:low | - |
| #195 | Projection: drop whole entities per audience (project-level filtering) | enhancement, area:cli, value:medium, effort:medium, priority:low | - |
| #154 | Clear RUSTSEC-2024-0320 (yaml-rust unmaintained) once syntect drops it upstream | area:ci, value:low, effort:low, priority:low | - |
| #129 | Decide whether to extract a ferrocv-core library crate | question, area:cli, value:low, effort:medium, priority:low | - |
| #117 | Earn OpenSSF Best Practices badge | area:ci, value:low, effort:low, priority:low | - |
| #101 | Theme adapter: neat-cv (Typst Universe) | area:themes, value:medium, effort:high, priority:low | v0.10.0 |
| #100 | Theme adapter: cobalt-cv (Typst Universe) | area:themes, value:medium, effort:medium, priority:medium | v0.10.0 |
| #99 | Theme adapter: lavandula (Typst Universe) | area:themes, value:medium, effort:medium, priority:medium | v0.10.0 |
| #98 | Theme adapter: moderner-cv (Typst Universe) | area:themes, value:medium, effort:medium, priority:medium | v0.10.0 |
| #97 | Theme adapter: impressive-impression (Typst Universe) | area:themes, value:medium, effort:low, priority:medium | v0.10.0 |
</untrusted-issue-data>

## Open milestones
<untrusted-issue-data>
- Phase 0: Project scaffolding [closed] open=0 closed=6 due=null — Remaining CI/tooling work before MVP.
- Phase 1: MVP — validate + render PDF [closed] open=0 closed=5 due=null — Bundle schema, implement validate, embed Typst, first theme adapter, end-to-end render.
- Phase 3: Theme adapters [closed] open=0 closed=6 due=null — Three theme adapters (basic-resume, modern-cv, fantastic-cv), adapter-pattern contributor docs, and theme resolution (local path, bundled, Typst Universe package).
- Phase 2: Multi-format output [closed] open=0 closed=3 due=null — HTML output via Typst HTML export, plain text output, and a decision on DOCX strategy.
- v0.8.0 [closed] open=0 closed=7 due=null — Targeted projection (CONSTITUTION §7) — the differentiating headline. Maintain one master resume.json, emit audience-specific cuts. Gated on two ADRs before implementation.
- v0.9.0 [closed] open=0 closed=7 due=null — Theme breadth — additional Typst Universe adapters and the native theme contract (CONSTITUTION §4).
- v0.10.0 [open] open=5 closed=0 due=null — Theme breadth — additional Typst Universe adapters built on top of the native theme contract (CONSTITUTION §4).
</untrusted-issue-data>

## Recent activity (last 6 months)
- Commits: 352
- Last 30 commit subjects:
```
a2a909d Merge pull request #245 from cacack/fix/rustls-advisory
ca8f173 fix(deps): bump rustls to 0.23.45 for RUSTSEC-2026-0285
9ebe866 Merge pull request #241 from cacack/dependabot/github_actions/github-actions-5b4f4fc240
e0fca19 chore(deps): bump the github-actions group with 4 updates
6030564 Merge pull request #239 from cacack/dependabot/github_actions/github-actions-54efbb11a2
3609cfb chore(deps): bump the github-actions group with 4 updates
a82f5a3 Merge pull request #238 from cacack/dependabot/github_actions/github-actions-b0e723f124
a3987ad chore(deps): bump the github-actions group with 7 updates
e42e28e Merge pull request #215 from cacack/dependabot/cargo/clap-4.6.4
f1ec13a chore(deps): bump clap from 4.6.1 to 4.6.4
d0fe1c8 Merge pull request #222 from cacack/dependabot/cargo/serde_json-1.0.151
6b01d85 chore(deps): bump serde_json from 1.0.150 to 1.0.151
eadc57c Merge pull request #223 from cacack/dependabot/cargo/typst-0.15.1
f740b53 chore(deps): bump typst from 0.15.0 to 0.15.1
0334191 Merge pull request #227 from cacack/dependabot/cargo/jsonschema-0.49.2
d5a39cb chore(deps): bump jsonschema from 0.46.8 to 0.49.2
c4cb2c2 Merge pull request #231 from cacack/ci/229-shared-dependabot-automerge
4404f39 ci: map the two secrets by name instead of inheriting
f042d81 ci: grant pull-requests to the called workflow, adopt v2.0.0
cff82bd Merge pull request #232 from cacack/dependabot/github_actions/github-actions-dae1a59564
067b823 chore(deps): bump taiki-e/install-action in the github-actions group
5f34dae ci: pin the shared workflow by commit SHA
61382ba ci: pin the shared workflow to the immutable v1.1.0
7628ba0 ci: call the shared dependabot-automerge workflow
c6fda69 Merge pull request #228 from cacack/dependabot/github_actions/github-actions-a560970f00
9057171 chore(deps): bump the github-actions group with 5 updates
463e760 Merge pull request #226 from cacack/ci/group-dependabot-actions
c0febc0 ci: group minor/patch GitHub Actions bumps into one Dependabot PR
d3ce000 Merge pull request #225 from cacack/dependabot/github_actions/ossf/scorecard-action-2.4.4
da70801 Merge pull request #224 from cacack/dependabot/github_actions/taiki-e/install-action-2.85.4
```
- Recent releases (from `gh release list`; local tags not fetched):
```
v0.9.1 2026-10-02 (latest)
v0.9.0 2026-06-26
v0.8.0 2026-06-15
v0.7.0 2026-06-01
v0.6.0 2026-05-11
v0.5.0 2026-04-20
v0.4.0 2026-04-20
v0.3.0 2026-04-19
v0.2.1 2026-04-19
v0.2.0 2026-04-19
```

## Other top-level docs
- SECURITY.md: present
- CONTRIBUTING.md: absent
- CODE_OF_CONDUCT.md: absent
- CHANGELOG.md: present
- ROADMAP.md: absent
- CLAUDE.md: present
