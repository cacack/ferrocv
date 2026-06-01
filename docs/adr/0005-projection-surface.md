# 0005. Ship projection as both a standalone `tailor` subcommand and `render` flags over one transform

**Status:** Proposed
**Date:** 2026-05-31

## Context

Issue [#147] is the second of the two gating ADRs for the
targeted-projection epic ([#17], [`CONSTITUTION.md`][const] §7). Its
sibling [0004] fixes the on-document tag schema; this ADR fixes the
**CLI surface** through which a user invokes projection.

Projection is a distinct stage upstream of rendering: it takes the
master `resume.json` plus a selection spec (the `--audience` curated
tag from [0004]/[#149], plus the mechanical `--since` / `--max-bullets`
/ `--redact` filters from [#148]) and produces a **derived document
that is itself valid JSON Resume**, which then flows into the existing
render pipeline unchanged.

The question is how the user reaches that stage:

- A **standalone subcommand** — `ferrocv tailor master.json --audience
  security -o cut.json` — that emits the derived JSON Resume as an
  inspectable artifact, which the user then `render`s separately.
- **Render-time flags** — `ferrocv render master.json --audience
  security --theme X -o out.pdf` — that run projection implicitly and
  go straight to output, no intermediate file.
- **Both**, sharing one projection implementation.

The relevant constraints, from [`CONSTITUTION.md`][const]:

- **§7 Projection is a distinct stage that emits valid JSON Resume.**
  The derived document being *itself valid JSON Resume* is a stated
  property, not an implementation detail — which makes "let the user
  hold that document in their hand" a natural and cheap affordance, not
  a feature to engineer.
- **§5 Simple now; iterate later.** Two entry points are only justified
  if they're two thin faces on *one* implementation. If "both" meant
  two code paths, §5 would push to one; because the projection
  transform is a single function either surface calls, the marginal
  cost of the second face is a clap arg group and a writer.
- **§4 Selection lives in Rust, never in themes.** Whichever surface(s)
  we expose, the transform is Rust preprocessing; the renderer receives
  an already-narrowed document. The surface choice does not touch the
  theme contract.
- **§1 The master is consumed unmodified.** Both surfaces read the
  master read-only and write *new* outputs (a derived `.json`, or a
  rendered artifact); neither mutates `master.json` in place.

The epic's own framing already leans toward "both — a tailor stage
invocable standalone *and* implicitly by `render`." This ADR confirms
that and pins down the relationship between the two so [#148]/[#149]
build one transform, not two.

## Decision

**Ship both surfaces over a single projection transform.** The two are
defined to be equivalent, so that:

```
ferrocv render master.json --audience security --theme X -o out.pdf
```

is, by construction, equivalent to:

```
ferrocv tailor master.json --audience security \
  | ferrocv render --theme X -o out.pdf
```

The piped form relies on the existing CLI convention that `render`
(and `validate`) read their input document from **stdin when the path
argument is omitted** (`src/cli.rs`), and that `tailor` writes its
derived document to stdout when `-o` is omitted (below). No new
stdin-handling is required by this ADR — the pipe composes from
conventions already in the binary. (Note: the input is selected by
*omitting* the path, not by a literal `-`; `render -` would try to open
a file named `-`.)

**What "equivalent" means, precisely.** The equivalence is asserted on
the **intermediate derived JSON document**, not on the final rendered
artifact: the document `render --audience` builds in memory is
*structurally equal* (same JSON value — modulo key ordering and
whitespace) to what `tailor` emits. It is deliberately **not** a claim
of byte-identical rendered output — a rendered PDF is not reproducible
byte-for-byte across invocations (Typst embeds non-deterministic data),
so the invariant lives at the JSON layer where it is both true and
testable. The [#148]/[#149] scenario test therefore compares the two
derived JSON documents structurally, not the PDFs byte-wise.

### `ferrocv tailor`

A new subcommand that runs the projection stage and **stops** —
emitting the derived valid JSON Resume.

- `ferrocv tailor <master.json> [projection flags] [-o <cut.json>]`
  (master path omitted ⇒ read from stdin, matching `render`/`validate`).
- Writes the derived document to `-o <path>`, or to **stdout** when
  `-o` is omitted or `-o -`, so it composes in a pipe.
- **stdout carries only the JSON document; all diagnostics go to
  stderr, unconditionally (no TTY detection).** So `tailor … | …` is
  always clean to parse, and a human running it interactively still
  gets a parseable dump rather than mixed output.
- The output is plain JSON Resume with `x-ferrocv` tags resolved and
  stripped per [0004]; it re-validates against the embedded schema
  ([#150]).
- Does **not** render. Format/theme flags are not accepted here; its
  job ends at the derived document.

### `ferrocv render` projection flags

`render` gains the same projection flags. When any are present, it runs
the identical transform on the input, then renders the derived document
as it would any input.

- `ferrocv render <master.json> [projection flags] --theme X --format F -o out.F`
- With **no** projection flags, `render` behaves exactly as today (no
  projection stage runs; the input flows straight through). Projection
  is opt-in and inert by default.

### Shared flag definition

The projection flags — `--audience <name>` (curated, [#149]) and the
mechanical `--since` / `--max-bullets` / `--redact` ([#148]) — are
defined **once** as a shared clap arg group and attached to both
subcommands. Both call the same `project(document, spec) -> Document`
function. There is one transform, one set of flag semantics, two entry
points.

**`--audience` takes exactly one value in v1; repeating it is a usage
error.** Even though an item's `x-ferrocv.audience` tag is multi-valued
([0004]), selecting *across* multiple audiences in one invocation
(union vs. intersection) is deferred to a future [#149] ADR — pinning
this here closes the gap [0004] points at and stops the flag from
silently accepting a second value with undefined meaning. The
`--redact` value vocabulary and the mechanical flags' exact semantics
are [#148]'s to define.

## Alternatives considered

**A. Both surfaces over one transform (chosen).** See *Decision*.

  - *Why it was attractive:* `tailor` gives an inspectable,
    diff-able, version-controllable derived artifact — you can eyeball
    "did the security cut keep the right bullets?" before committing to
    a PDF, and even commit the cut. `render --audience` gives the
    one-shot path for the common case (master → tailored PDF) without
    an intermediate file to manage. The §7 "derived doc is valid JSON
    Resume" property means `tailor` is nearly free — the document
    already exists mid-pipeline; we just let the user stop there.
  - *Why chosen:* the two faces share one transform, so §5's "don't
    build two things" objection doesn't apply — the second face is a
    flag group and a JSON writer. It serves both the
    inspect-then-render workflow and the quick one-shot, and the
    pipe-equivalence (`render --audience X` ≡ `tailor … | render`)
    keeps them honest: any divergence is a bug, not a design seam.

**B. Standalone `tailor` subcommand only (rejected).** Projection is
*only* reachable via `tailor`; to get a tailored PDF you always run two
commands (`tailor` then `render`).

  - *Why it was attractive:* maximally explicit — the derived document
    is always a real, inspectable file; there's exactly one way to
    project; `render` stays a pure renderer with zero projection
    surface.
  - *Why rejected:* it taxes the common case. The frequent operation is
    "master → security PDF"; forcing a temp file and a second
    invocation for every cut is friction with no payoff when the user
    doesn't care to inspect the intermediate. We can keep the
    inspectability benefit (it's just `tailor`) *and* offer the
    one-shot, so there's no reason to withhold the latter.

**C. Render-time flags only (rejected).** Projection lives solely on
`render`; there's no standalone derived artifact unless we later bolt on
a `--dump-projection <path>` debug flag.

  - *Why it was attractive:* one subcommand, no new command surface;
    the simplest possible CLI.
  - *Why rejected:* it hides the thing §7 deliberately made a
    first-class value — the derived document *is* valid JSON Resume,
    and being able to inspect/commit it is much of the point of a
    "single master, many cuts" workflow. Reducing that to a debug-only
    side-channel undersells the design. And once you add
    `--dump-projection` to recover it, you've reinvented `tailor` as a
    flag, more awkwardly.

**D. A piped-only / library-only transform with no dedicated UX
(rejected).** Expose projection purely as something `render` does, plus
a documented "if you want the JSON, here's the internal API" — no
`tailor`, no dump.

  - *Why rejected:* this is C without even the escape hatch; it makes
    the inspectable derived document unreachable from the CLI, which
    contradicts §7's intent. Not seriously in contention; listed for
    completeness.

## Consequences

**Positive.**

- Two workflows, both first-class: *inspect-then-render* (`tailor` →
  review/commit the cut → `render`) and *one-shot* (`render
  --audience`). Neither is a second-class afterthought.
- The pipe-equivalence is a built-in correctness invariant and a tidy
  test target: `render --audience X` must equal `tailor … | render`,
  *structurally on the intermediate JSON* (see *Decision* — not on the
  rendered PDF). [#148]/[#149] scenario tests assert it directly.
- `tailor`'s stdout-by-default makes projection composable with other
  JSON tooling (`jq`, validation, diffing two cuts) for free.
- Theme contract untouched (§4): the renderer always receives an
  already-narrowed valid JSON Resume regardless of which surface ran
  the transform.

**Negative.**

- **Two places to document the same flags.** `--audience` and the
  mechanical filters appear on both `tailor` and `render` help. The
  shared arg group keeps them in lockstep in code, but user docs and
  `--help` now describe the projection flags in two contexts. Modest,
  accepted.
- **A surface for flag drift if the arg group is ever forked.** The
  whole design rests on *one* transform behind *one* flag group;
  someone adding a projection flag to only one subcommand would break
  the pipe-equivalence. The equivalence test (above) is the guardrail,
  and the invariant is stated here so the temptation is visible.
- **Slightly larger CLI surface** — one new subcommand plus flags on an
  existing one. Justified by §7's first-class derived document; the
  cost is real but small.
- **`tailor`'s stdout-by-default is a deliberate call on the §6
  surface.** §6 frames ferrocv's data flow as "we read `resume.json`,
  we write files to disk the user specified"; stdout is an *additional*
  output channel beyond a named file. We judge it within the spirit of
  §6, not a weakening of it: the user explicitly invoked `tailor` with
  no `-o`, choosing stdout themselves; the derived document never leaves
  the local process tree except where that user's own shell directs it;
  and it adds no network path and no implicit persistence. The PII
  exposure that remains is the ordinary Unix one — full resume content
  printed to the terminal in a shared, recorded, or CI context — so the
  `tailor --help` text notes that stdout carries full resume PII and
  that `-o <file>` is the safer default for unattended use. This is a
  UX note, not a §6 amendment; no constitutional change is implied.

**Non-goals that fall out.**

- No projection *config file* (audience→theme→format mappings) in v1.
  The epic flags the "one master → many (audience × theme × format)
  outputs without a config explosion" question as open; this ADR
  deliberately does **not** answer it. v1 is one cut per invocation via
  flags; batch/config orchestration is a later decision (§5).
- No auto-fit-to-page-count on either surface — a stated §7 non-goal.
- No in-place mutation of `master.json` from either surface (§1).
- No `render`-time projection without explicit flags — projection is
  opt-in; a flagless `render` is unchanged.

**Revisit if.**

- The "many cuts in one go" need (audience × theme × format batches)
  materializes enough to justify a config or manifest surface — that's
  its own ADR, gated on real usage, not pre-built here.
- The two-place flag documentation or the drift risk proves to be a
  real maintenance burden rather than a theoretical one.

## References

- Issue: [#147] — ADR: projection surface (`tailor` subcommand vs.
  render flags vs. both)
- Epic: [#17] — targeted projection
- Sibling ADR: [0004] — audience-tag schema under `x-ferrocv`
- Blocks: [#148] (mechanical filters), [#149] (curated `--audience`),
  [#150] (derived re-validation), [#151] (docs)
- [`CONSTITUTION.md`][const] §1, §4, §5, §6, §7

[#17]: https://github.com/cacack/ferrocv/issues/17
[#147]: https://github.com/cacack/ferrocv/issues/147
[#148]: https://github.com/cacack/ferrocv/issues/148
[#149]: https://github.com/cacack/ferrocv/issues/149
[#150]: https://github.com/cacack/ferrocv/issues/150
[#151]: https://github.com/cacack/ferrocv/issues/151
[0004]: ./0004-audience-tag-schema.md
[const]: ../../CONSTITUTION.md
