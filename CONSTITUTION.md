# Constitution

The non-negotiables for `ferrocv`. This document answers **what** we're
building and **why**. *How* lives in the code and `TODO.md`.

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
  embedded Typst `World` actively rejects any `@preview/...` package
  import rather than fetching it — this rejection is hard and is not
  relaxed by any feature flag or subcommand. Rendering may read from
  the local installer cache populated by a prior
  `ferrocv themes install` (see next bullet); that is a local
  filesystem read, not a network call, and does not weaken the
  `render`-is-offline guarantee.
- **`ferrocv themes install` is the single, enumerated network-permitted
  entry point.** It is an explicit, user-initiated subcommand that
  fetches only from the Typst Universe `@preview` registry over HTTPS
  (`https://packages.typst.org/preview/<name>-<version>.tar.gz`); it
  is never invoked transitively from `render` or `validate`; its
  network-capable dependencies live behind a Cargo feature flag
  (`install`) so the default build contains no network code at all.
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

## Non-goals

These are deliberately out of scope. Proposals to add them belong in an
amendment PR, not a feature PR.

- Replacing, forking, or extending the JSON Resume schema.
- Supporting input formats other than JSON Resume (Markdown, YAML,
  HR-XML, etc.).
- Becoming a general-purpose Typst build tool.
- Shipping a hosted service, web UI, or SaaS wrapper.

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
