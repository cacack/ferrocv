# 0002. Silently ignore unknown JSON Resume `x-*` extension fields

**Status:** Proposed
**Date:** 2026-05-17

## Context

[`CONSTITUTION.md`][const] §1 reserves the `x-<namespace>` prefix as
the only extension mechanism for JSON Resume input: anything not
expressible in stock JSON Resume goes under `x-*` fields that themes
may opt into. The constitution does not say what happens to an `x-*`
field that no theme — or *this* theme — knows about.

Today (Phase 2), the registry holds six themes and **zero of them
consume any `x-*` field**. The vendored schema at
`assets/schema/jsonresume-v1.0.0.json` already permits unknown
properties on the major objects via `additionalProperties: true`, so
`ferrocv validate` does not reject them. There is no `x-*` handling
code in `src/`, `tests/`, or `assets/themes/`.

The decision needs to land before adapter count grows past one or two:
once we have an adapter that consumes (say) `x-vcard.role` and another
that doesn't, users who carry the same `resume.json` across themes
need to know what to expect. Issue [#131] frames the question as
*silent drop vs. warn*.

The relevant constraints, all from [`CONSTITUTION.md`][const]:

- **§1 JSON Resume is the canonical input.** The `x-` prefix exists
  *because* themes need a private channel for theme-specific data. The
  expected steady state is that most themes ignore most `x-*` fields.
- **§5 Simple now; iterate later.** With zero themes consuming any
  `x-*` field today, any introspection or registration surface is
  pre-engineering for a caller that does not exist.
- **§3 Multi-format output is first-class.** Whatever we choose must
  behave identically across PDF, HTML, and plain text — the user
  shouldn't get a warning rendering to one format and silence in
  another.

## Decision

**Silently ignore unknown `x-*` fields at render time.** `ferrocv
validate` continues to accept them (the schema already does); `ferrocv
render` does not enumerate which `x-*` fields a theme consumed and
does not warn about ones it didn't. A theme that wants an `x-*` field
reads it directly; a theme that doesn't never sees it. There is no
opt-in registration, no `--strict-extensions` flag, and no per-render
diagnostic.

## Alternatives considered

**A. Silent drop (chosen).** Validation accepts `x-*` fields; themes
that opt in consume them; themes that don't ignore them; no
diagnostics either way.

  - *Why it was attractive:* matches the design intent of §1 — `x-*`
    is a private channel between a user and a specific theme. Renders
    cleanly across themes from a single shared `resume.json`. Zero
    new surface area for a problem we don't yet have.
  - *Why chosen:* per §5, this is the narrowest solution that meets
    the requirement, and the requirement today is just *"don't blow
    up."*

**B. Warn on every unconsumed `x-*` field.** `render` walks the input,
collects `x-*` field paths the theme didn't touch, and prints
`warning: theme "X" does not consume x-foo.bar` to stderr per field.

  - *Why it was attractive:* catches typos (`x-vcrad.role` instead of
    `x-vcard.role`); makes the "this field was ignored" outcome
    visible rather than silent.
  - *Why rejected:* the steady-state signal-to-noise is bad. A user
    with a `resume.json` carrying three or four `x-*` fields rendered
    across four themes would get most of those warnings on most
    renders, by design. CI logs and re-render loops would fill with
    diagnostics that describe normal, intended behavior. The typo case
    is real but small, and it has a better fix (an explicit `ferrocv
    lint` subcommand whose noise the user opted into) than warning on
    every render.

**C. Opt-in theme introspection.** Each theme declares the set of
`x-*` namespaces it consumes; `render` warns only on `x-*` fields
whose namespace prefix no registered theme consumes (or, in a `--strict`
variant, that *this* theme doesn't consume).

  - *Why it was attractive:* lower-noise version of B that still
    catches the typo case.
  - *Why rejected:* this is exactly the pre-engineering §5 warns
    against. We'd be designing a theme-metadata surface — what the
    declaration looks like, where it lives (theme source? a sidecar
    manifest?), how it's checked, how adapters that wrap upstream
    templates *we don't control* declare what they consume — before
    a single caller needs it. When a theme actually opts into an
    `x-*` field, that will be the moment to decide whether
    introspection earns its keep.

**D. `ferrocv lint` subcommand (deferred, not chosen as part of this
ADR).** A separate command — explicitly opted into — that walks
`resume.json` and reports `x-*` fields no bundled theme would consume,
plus typo heuristics (`x-vcrad` close to `x-vcard`). Not part of this
decision because there's nothing to lint against yet, but it's the
natural home for the "I want to know about my typos" use case if
demand materializes.

## Consequences

**Positive.**

- A single `resume.json` carrying theme-specific `x-*` fields renders
  cleanly across every theme without per-render noise — supporting
  the §1 portability promise.
- Themes that consume `x-*` fields just read them; themes that don't
  do nothing. No registration ceremony, no theme-metadata surface to
  design or maintain.
- Identical behavior across PDF, HTML, and plain text (§3) falls out
  for free: nothing to do per format.
- Zero new code: this ADR is the entire change.

**Negative.**

- **Typos in `x-*` field names are silent.** `x-vcrad.role` will be
  ignored exactly like `x-vcard.role` would have been if the theme
  didn't consume it. Users learn about the mismatch by noticing
  missing output. This is a real cost; we accept it for v1 in
  exchange for the simpler model, and revisit if the typo case
  becomes a recurring user complaint.
- **No on-render signal that an `x-*` field was used.** If a theme
  *does* consume an `x-*` field, there's no audit trail saying so
  beyond "the rendered output contains the value." For Phase 2 this
  is fine; if it becomes a debugging pain, the `lint` subcommand (or
  a `--verbose` render flag) can address it without changing the
  default behavior.

**Non-goals that fall out.**

- No warnings, no diagnostics, no logs about unconsumed `x-*` fields
  at render time.
- No theme-metadata surface for declaring consumed `x-*` namespaces.
- No `--strict-extensions` or equivalent render flag.

**Revisit if.**

- Typos in `x-*` field names become a recurring user complaint that
  the (then-to-be-built) `ferrocv lint` subcommand cannot adequately
  address.
- Multiple themes converge on a shared `x-*` namespace convention
  significant enough that cross-theme contracts (rather than
  per-theme private channels) become the dominant use case — at that
  point `x-*` is no longer a "private channel" and the design
  premise of this ADR shifts.

## References

- Issue: [#131] — Decide handling of unknown JSON Resume `x-*`
  extension fields
- [`CONSTITUTION.md`][const] §1, §3, §5

[#131]: https://github.com/cacack/ferrocv/issues/131
[const]: ../../CONSTITUTION.md
