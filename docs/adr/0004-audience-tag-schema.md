# 0004. Carry audience tags under a single `x-ferrocv` namespace; untagged content is universal

**Status:** Proposed
**Date:** 2026-05-31

## Context

Issue [#146] is one of the two gating ADRs for the targeted-projection
epic ([#17], [`CONSTITUTION.md`][const] §7). Projection lets a user
maintain one comprehensive master `resume.json` and emit
audience-specific cuts — the headline being *curated* selection:
`--audience security` keeps the content **tagged** for that audience,
not the first N entries by position. Before any of the implementation
issues ([#148]–[#151]) can start, two things must be decided:

1. **The `x-` field shape** that carries audience tags, and at what
   **granularity** (whole `work` entries vs. individual `highlights`
   vs. whole sections).
2. **The untagged-content default** — when `--audience` is active, does
   content with no tag get *included* or *excluded*?

This ADR settles both. The sibling ADR [0005] (proposed concurrently)
settles the CLI surface; together they unblock [#148]–[#151].

The relevant constraints, all from [`CONSTITUTION.md`][const]:

- **§1 JSON Resume is the canonical input.** The master is consumed
  *unmodified*; audience metadata may only ride under the `x-<namespace>`
  extension prefix. We do not add new top-level keys, change field
  semantics, or fork the schema. The vendored schema already permits
  unknown properties via `additionalProperties: true`, and ADR [0002]
  governs how unknown `x-*` fields are handled.
- **§7 Projection selects and omits; it never rewrites or generates.**
  This is the sharpest constraint on the tag *representation*. Any
  scheme that requires editing the displayed prose to carry a tag — an
  inline marker like `Led the SOC migration {#security}` that
  projection strips before rendering — is a **content rewrite** and is
  out of bounds. Tags must live *beside* content, never *inside* it.
- **§5 Simple now; iterate later.** Pick the narrowest representation
  that delivers the headline (per-highlight curated selection). Do not
  pre-build granularities or defaults for callers that don't exist yet.
- **§4 Selection lives in Rust, never in themes.** The tag is consumed
  by the projection stage; themes never see it. Whatever shape we
  choose, the derived document that reaches the renderer must be plain
  valid JSON Resume with the tags already resolved away.

A schema wrinkle forces part of the decision. JSON Resume
`work[].highlights` (and `projects[].highlights`, etc.) are **arrays of
bare strings**, not objects:

```json
"highlights": ["Led the SOC migration", "Mentored 4 engineers"]
```

You cannot attach an `x-` field to a string. So per-highlight tagging —
which *is* the headline ("keep the tagged highlights, not the first N
by position", explicitly contrasting with the mechanical
`--max-bullets N` of [#148]) — cannot be expressed inline without
either changing the string (forbidden by §7) or promoting highlights to
objects (forbidden by §1). The only §1/§7-clean option left is a
sibling structure on the parent object, keyed to the highlight by
array index.

## Decision

**All ferrocv projection metadata lives under a single `x-ferrocv`
object on the node it applies to. An item with no tag is universal
(included in every cut); a tag *restricts* its item to the listed
audiences. Two granularities ship in v1: array-element objects and
individual highlights.**

### Field shape

A single namespaced object, `x-ferrocv`, groups all of this tool's
extension metadata under one key (rather than scattering several
`x-ferrocv-*` siblings). It carries two fields, both optional:

- `audience` — a string array tagging the **enclosing object**
  (a `work` entry, a `project`, a `volunteer` entry, an `award`, a
  `skills` item, …).
- `highlights` — an array **index-parallel** to the enclosing object's
  `highlights` array, where entry *i* is the audience-tag list for
  `highlights[i]`.

```json
{
  "name": "Acme Corp",
  "position": "Security Engineer",
  "x-ferrocv": {
    "audience": ["security", "leadership"],
    "highlights": [
      ["security"],
      ["leadership"],
      []
    ]
  },
  "highlights": [
    "Led the SOC migration",
    "Mentored 4 junior engineers",
    "Cut cloud spend 30%"
  ]
}
```

### Matching semantics

For a selection `--audience X`, an item is **kept** iff it is *untagged*
**or** its tag list contains `X`; it is **dropped** only when it is
*tagged and `X` is absent*. Concretely:

- `x-ferrocv.audience` absent, or absent/`[]` at index *i* of
  `x-ferrocv.highlights` → **universal**, kept in every cut.
- `x-ferrocv.audience: ["security"]` → kept for `--audience security`,
  dropped for `--audience leadership`.
- A whole `work` entry dropped by its `audience` tag takes its
  highlights with it; per-highlight tags only matter for entries that
  survive.

**An empty array always means universal, never "exclude from all."**
Both `x-ferrocv.audience: []` on an object and `[]` at a highlight slot
are exactly equivalent to the tag being absent: the item is kept in
every cut. We reserve this so that a user who hand-authors `[]` (to
document "this is for everyone") can never have it silently invert. A
future opt-in strict/exclude mode must spell exclusion with a *distinct*
sentinel (decided by that mode's own ADR) — `[]` is permanently pinned
to "universal" here.

**A length mismatch is a hard error, not a silent fallback.** If
`x-ferrocv.highlights` is present and its length differs from the
enclosing entry's `highlights` length (or it carries an out-of-range
index), projection MUST fail with a user-visible error that names the
offending entry — it MUST NOT pad, truncate, or silently treat the
unaligned tail as universal. This is required behavior, enforced as part
of [#150]'s re-validation; it is not an optional lint. Rationale: a
positional array silently misattributing tags after a reordered or
inserted bullet is the schema's sharpest footgun (see *Consequences*),
and a resume that silently ships the wrong cut is worse than one that
refuses to build.

The tag is multi-valued: an item can belong to several audiences. In
v1, `--audience` selects exactly one audience and repeating the flag is
a usage error (pinned in [0005]); union/intersection over *multiple*
audiences in one invocation is left to a future [#149] ADR. This ADR
fixes only the on-document shape and the single-audience matching rule.

### Granularity: array-element objects + highlights (v1)

Two levels are tagging surfaces in v1:

- **Array-element objects** — any object in a JSON Resume array
  (`work[]`, `volunteer[]`, `projects[]`, `education[]`, `awards[]`,
  `publications[]`, `skills[]`, `languages[]`, `interests[]`,
  `references[]`) carries tags via its own `x-ferrocv.audience`.
- **Highlights** — any JSON Resume object that defines a bare-string
  `highlights` array is tagged via that parent's index-parallel
  `x-ferrocv.highlights`. In stock JSON Resume v1.0.0 those objects are
  `work[]`, `volunteer[]`, and `projects[]`. On an object with no
  `highlights` field, an `x-ferrocv.highlights` key is meaningless and
  is ignored (it has nothing to align to).

**Whole-section tagging is deferred.** "Drop the entire Awards section
for the security cut" is already expressible by tagging each element of
that section, because every JSON Resume section except `basics` is an
array of objects — and you would never drop `basics` wholesale. A
dedicated top-level section toggle (e.g. an `x-ferrocv.sections` map on
the root) is a third representation surface we do not need yet (§5); it
gets its own ADR if a real "toggle a section as a unit without touching
its elements" need shows up.

**Suppressing PII fields *within* `basics` is out of scope for audience
tagging** — it is the job of the mechanical `--redact` filter ([#148]).
`basics` is a singleton object, not an array, so it carries no
audience tag and the include-by-default rule keeps its fields (name,
address, phone, email) in every cut. Redacting `basics.address` /
`basics.phone` for a public cut is therefore *not* something `--audience`
does; it is a separate, named-field-path transform that `--redact` runs
on the derived document after audience selection. [#148] owns that
filter's field-path scope and a negative test asserting the redacted
fields are absent. This ADR notes the boundary only so the two
mechanisms don't get conflated: tags select array content; `--redact`
suppresses named PII fields.

### Projection strips what it consumed

The derived document must re-validate as JSON Resume ([#150]). It will,
because `x-*` is permitted — but leaving spent `x-ferrocv` tags in the
cut is noise. The projection stage **removes the `x-ferrocv` keys it
consumed** from the derived output. This is not a §7 content rewrite:
`x-ferrocv` is control metadata the user authored for the tool, not
resume prose. The user's *master* keeps its tags untouched (§1); only
the *derived* document is cleaned.

This stripping is a **required, test-gated invariant**, not just stated
intent: [#150] carries a negative test asserting that no `x-ferrocv` key
appears anywhere in the derived document. Beyond tidiness it is a small
privacy property — an un-stripped cut would carry the user's full
audience-targeting topology (the names of every audience, and which
items they hid from whom) to whoever receives the document, which under
[0005]'s `tailor` surface can be a recruiter or an ATS.

## Alternatives considered

**Untagged default — exclude instead of include (rejected).** Only
content tagged with the matching audience survives; untagged content is
dropped.

  - *Why it was attractive:* stricter curation — the cut contains
    exactly what you marked, nothing leaks in by forgetting to tag.
  - *Why rejected:* it punishes the gradual-adoption path and is
    dangerous for the specific data type. A user pointing `--audience
    security` at a freshly-tagged master (where they've tagged three
    specialty highlights and nothing else) would get a near-empty
    resume — *whole jobs silently omitted* because they lack a tag.
    Omitting a job is a far worse failure than including one
    marginally-relevant bullet. Include-by-default makes "untagged
    master + `--audience`" a safe no-op and lets users tag
    incrementally, tightening cuts as they go. A future opt-in strict
    mode (`--strict`) can layer on top without changing this default —
    but per §5 we don't build it now. Note such a mode must *not*
    overload `[]` to mean "exclude": *Matching semantics* permanently
    pins `[]` to "universal", so strict exclusion needs its own
    sentinel, decided by that mode's ADR.

**Inline marker inside the highlight string (rejected).** Carry the tag
in the prose, e.g. `"Led the SOC migration {#security}"`, and strip it
during projection.

  - *Why it was attractive:* no index-parallel array to keep aligned;
    the tag travels with the exact string it annotates, immune to
    reordering.
  - *Why rejected:* it **violates §7**. Stripping `{#security}` from the
    rendered bullet is rewriting displayed content — exactly the line
    §7 draws ("selects and omits; never rewrites"). It also reads as a
    bespoke micro-dialect embedded in a schema field, brushing against
    §1. The §7 conflict alone is disqualifying.

**Promote highlights to tagged objects (rejected).** Represent each
highlight as `{ "text": "...", "x-ferrocv": {...} }` instead of a bare
string.

  - *Why it was attractive:* every highlight becomes a normal tagging
    surface, uniform with array elements; no index bookkeeping.
  - *Why rejected:* JSON Resume defines `highlights` as `string[]`. An
    object-valued highlight is not valid JSON Resume — it forks the
    schema, violating §1, and would fail the master's own
    `ferrocv validate`. Non-starter.

**Scattered `x-ferrocv-audience` / `x-ferrocv-highlights` siblings
(rejected, minor).** Two flat top-level `x-` fields per node instead of
one nested `x-ferrocv` object.

  - *Why it was attractive:* marginally flatter to type; each field
    self-describes.
  - *Why rejected:* it spreads the namespace across multiple keys and
    multiplies the surface ADR [0002] has to reason about. One
    `x-ferrocv` object is tidier, documents as a single unit, and
    leaves room for future ferrocv metadata without minting new
    top-level keys. Purely an ergonomics/hygiene call, but it costs
    nothing.

**Section-level tagging in v1 (deferred, not rejected).** A root-level
`x-ferrocv.sections` map (`{"awards": ["leadership"]}`) toggling whole
sections.

  - *Why it was attractive:* one place to say "no awards in the security
    cut" without touching each award.
  - *Why deferred:* element-level tags already express this for every
    array section, so it's a third representation buying a convenience
    we have no evidence anyone needs yet (§5). It earns its own ADR if
    a genuine whole-section-as-a-unit need appears.

## Consequences

**Positive.**

- The headline — curated per-highlight selection — is representable in
  v1 without forking the schema (§1) or rewriting content (§7). The
  index-parallel array is the only §1/§7-clean way to tag bare strings,
  and we take it.
- Include-by-default makes adoption incremental and safe: an untagged
  master renders identically with or without `--audience`, and no cut
  ever silently drops an entire job for lack of a tag.
- One `x-ferrocv` namespace object is a single thing to document, to
  consume in Rust, and to strip from the derived document — and a
  single anchor for any future projection metadata.
- The derived document is clean valid JSON Resume with tags resolved
  away, so it flows into the unchanged render/validate pipeline
  ([#150]) and themes stay ignorant of audiences (§4).

**Negative.**

- **`x-ferrocv.highlights` is positional and therefore fragile.** This
  is the accepted cost of JSON Resume modeling highlights as bare
  strings — there is no §1/§7-clean alternative. The *Decision* hardens
  the catchable half: length mismatches and out-of-range indices are a
  mandated hard error ([#150]), not a silent fallback. The residual
  footgun is the un-catchable half — a **length-preserving** reorder
  (swap two bullets, or insert one and delete another) leaves the array
  the right length but now pointing at the wrong strings. No structural
  check can detect that; it remains the accepted cost, and the tailoring
  guide ([#151]) should warn authors to re-check tags after reordering.
- **Typos in the `x-ferrocv` namespace are silent under ADR [0002].**
  [0002] silently ignores unknown `x-*` fields, so `x-ferovcv` (or
  `highlight` for `highlights`) drops the whole spec with no error — and
  include-by-default means the resulting cut looks plausible but is
  un-tailored, so the user may ship a master they meant to narrow.
  Unlike a third-party `x-*` field, `x-ferrocv` is *first-party* with a
  known shape, so this is fixable without the introspection surface
  [0002] declined: `ferrocv validate` (or a future lint) can recognize
  the `x-ferrocv` namespace specifically and check its internal shape.
  Recommended for [#150]; called out here so the gap is on the record.
- **No whole-section toggle in v1.** Dropping a section means tagging
  its elements. Acceptable because every list section's elements are
  individually taggable; revisited only if a real need appears.
- **Include-by-default can leak content into a cut** if the user
  expected exclude semantics — the inverse risk of the rejected
  alternative. We judge an over-full cut (user notices and adds tags)
  strictly safer than an under-full one (user ships a resume missing a
  job), and provide the future `--strict` escape hatch as the
  pressure-release valve.

**Non-goals that fall out.**

- No object-valued highlights, no new top-level keys, no schema fork
  (§1).
- No inline/in-prose tag syntax (§7).
- No root-level section map in v1.
- No strict/exclude mode in v1 (deferred, additive when needed).

**Revisit if.**

- Positional-highlight fragility becomes a recurring source of
  mistagged cuts that a lint check can't adequately catch — at which
  point a content-keyed (hash-based) or object-promotion-via-amendment
  scheme gets re-evaluated, the latter requiring a §1 amendment.
- Real demand appears for whole-section-as-a-unit toggling that
  element tags cannot express.
- Enough users want exclude/strict semantics by default that the
  include default is the wrong starting point rather than the safe one.

## References

- Issue: [#146] — ADR: audience-tag schema under `x-` (granularity +
  untagged default)
- Epic: [#17] — targeted projection
- Sibling ADR: [0005] — projection surface (`tailor` vs. render flags
  vs. both)
- Blocks: [#148] (mechanical filters), [#149] (curated `--audience`),
  [#150] (derived re-validation), [#151] (docs)
- Related ADR: [0002] — handling of unknown `x-*` extension fields
- [`CONSTITUTION.md`][const] §1, §4, §5, §6, §7

[#17]: https://github.com/cacack/ferrocv/issues/17
[#146]: https://github.com/cacack/ferrocv/issues/146
[#148]: https://github.com/cacack/ferrocv/issues/148
[#149]: https://github.com/cacack/ferrocv/issues/149
[#150]: https://github.com/cacack/ferrocv/issues/150
[#151]: https://github.com/cacack/ferrocv/issues/151
[0002]: ./0002-x-extension-field-handling.md
[0005]: ./0005-projection-surface.md
[const]: ../../CONSTITUTION.md
