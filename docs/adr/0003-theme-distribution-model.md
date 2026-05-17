# 0003. Theme distribution: in-repo curated set + Universe cache as the open extension surface

**Status:** Proposed
**Date:** 2026-05-17

## Context

Issue [#130] asks whether adapter themes should live in this repo or
split into per-theme repos under a `ferrocv-themes-*` namespace. The
question was deferred originally with the rule *"revisit when the
third or fourth adapter lands"*; today (Phase 2/3) the registry holds
six themes — four adapters (`typst-jsonresume-cv`, `fantastic-cv`,
`modern-cv`, `basic-resume`) and two native themes (`text-minimal`,
`html-minimal`) — and five more adapters ([#97], [#98], [#99], [#100],
[#101]) sit blocked behind this decision.

The framing has also shifted since the issue was written. When [#130]
was filed, "out-of-tree themes" was a hypothetical. Since then [#41]
landed `ferrocv themes install @preview/<name>:<version>` (Stage B),
which fetches Typst Universe packages into a local cache that the
render pipeline reads from. There are now effectively **three** places
a theme can come from:

1. **Bundled** — `include_bytes!`'d into the binary from
   `assets/themes/<name>/`.
2. **Universe cache** — fetched by `ferrocv themes install` and read at
   render time from the local cache (gated behind the `install` Cargo
   feature).
3. **Local path** — a single `.typ` file at a user-supplied path.

So the real question is no longer "in-repo vs. per-theme repos" as a
binary choice — it's "what is each of those three paths *for*, and do
we need a fourth?"

The relevant constraints, all from [`CONSTITUTION.md`][const]:

- **§4 Two theme interfaces, kept separable.** Adapters wrap upstream
  Typst Universe templates; native themes implement a contract
  directly against JSON Resume. This ADR is about *distribution*, not
  *interface*, but the two interact: an adapter's value proposition
  is "visual variety we didn't have to author"; the path that buys us
  the most variety with the least maintenance burden wins.
- **§5 Simple now; iterate later.** With ~200KB of total theme bytes
  in a 51MB release binary (themes are <0.5% of the artifact),
  pre-engineering a multi-repo distribution apparatus would be
  textbook §5 violation.
- **§6.1 No network calls in `render` or `validate`.** This bounds
  the design space: any distribution model must either bake the theme
  into the binary or route through the existing `themes install`
  installer cache. A "package manager that auto-fetches on render"
  is not on the table.
- **§6.4 Reproducible, verifiable releases.** Bundled themes are
  versioned with the binary; Universe-cached themes are versioned by
  the user via the spec they installed. Both are reproducible; they
  reproduce *different things* and that distinction matters.

The vendoring runbook ([`VENDORING_CHECKLIST.md`][checklist]) and the
modern-cv / basic-resume / fantastic-cv `VENDORING.md` files already
codify the in-repo adapter maintenance burden: `@preview/...` import
removal, empty-URL safety audits, license consistency, golden
regeneration. That work exists because we chose, *de facto*, to ship
adapters in-repo. This ADR is mostly making that *de facto* choice
*de jure* — and naming the second path (Universe cache) as the answer
to "what if a user wants a theme we didn't ship?"

## Decision

**Adapter and native themes ferrocv chooses to ship are bundled
in-repo under `assets/themes/<name>/`. The `themes install` Universe
cache is the open extension surface for themes ferrocv does not
ship.** There are no per-theme `ferrocv-themes-*` repos, no plugin
registry, no auto-discovery, no out-of-tree native-theme protocol. The
distinction between the two paths is editorial, not technical:

- **In-repo themes** are curated. We vendor the source, audit it
  against the `VENDORING_CHECKLIST.md` runbook, write the JSON-Resume
  glue, run them through the golden-file regression tests, and accept
  responsibility for them across `ferrocv` releases. Inclusion is a
  judgment call about visual range, license compatibility, and the
  maintenance cost we're willing to carry.
- **Universe-cache themes** are user-curated. A user runs `ferrocv
  themes install @preview/<name>:<version>`, accepts whatever quirks
  the upstream ships (including `@preview/...` transitive deps and
  whatever empty-URL behavior the author chose), and is responsible
  for upgrading when they want to. ferrocv guarantees only that the
  cache reader works.

The five queued adapter issues ([#97], [#98], [#99], [#100], [#101])
can ship as in-repo adapters under this model. New adapter proposals
beyond those go through normal triage: medium-effort themes that
expand visual range earn a spot; high-effort themes or themes that
duplicate range we already cover are politely directed to the
Universe-cache path.

## Alternatives considered

**A. Status-quo in-repo, declare Universe-cache out of scope as a
distribution model (rejected).** Keep shipping adapters in-repo and
treat `themes install` purely as a render-time convenience — not as
*the* answer for themes we don't ship.

  - *Why it was attractive:* simplest narrative. One source of truth
    for "what themes does ferrocv support" (the registry slice in
    `src/theme.rs`). Universe-cache themes become an
    implementation detail rather than a publicly-blessed extension
    path.
  - *Why rejected:* the cache path **already exists** post-[#41] and
    is documented in user-facing surfaces (`ferrocv themes install`
    is a subcommand, with `--help`). Pretending it isn't a
    distribution model is a fiction the code already contradicts. The
    honest framing — "here's the curated set, here's how to bring
    your own" — also defuses the eternal "will you add theme X?"
    pressure that motivates the per-theme-repo discussion in the
    first place.

**C. In-repo curated + Universe-cache extension (chosen).** What this
ADR codifies. See *Decision*.

  - *Why it was attractive:* makes explicit what the code already
    does. Gives a clear answer to "I want a theme you don't ship"
    (`ferrocv themes install`) and to "what does it cost ferrocv to
    add a theme?" (the vendoring runbook). No new infrastructure.
  - *Why chosen:* per §5, the narrowest decision that resolves the
    question. The infrastructure to support both paths already
    shipped; this ADR just names them.

**B. Per-theme `ferrocv-themes-<name>` repos (rejected).** Move each
adapter into its own crate or repo under a `ferrocv-themes-*`
namespace. The main binary ships with no adapters bundled; users add
themes by … some mechanism: a Cargo feature per theme, a runtime
plugin loader, a registry of approved repos the installer knows
about, etc.

  - *Why it was attractive:* independent versioning per theme (an
    upstream Typst Universe template bump touches only its own repo's
    release cadence). Smaller default binary (the ~200KB of vendored
    sources go away). Theoretical contributor ownership — a person
    could own `ferrocv-themes-fantastic-cv` without touching the
    main repo.
  - *Why rejected:* every cost is real and present-tense; every
    benefit is hypothetical and minor at the scale we operate at.
    - The 200KB savings is rounding error against a 51MB release
      binary (font assets dominate, not theme sources). Optimizing
      this is §5 pre-engineering for a problem we don't have.
    - Independent versioning is only valuable if upstream churn is
      high enough that bundled-with-ferrocv release cadence becomes
      a bottleneck. After a year of operation we have zero data
      points suggesting that.
    - "Contributor ownership" of a separate repo is a fiction unless
      we also build the discovery, trust, and review mechanisms that
      make those repos credible — a search index, a curation
      signal, a way to know that `ferrocv-themes-mystery` isn't
      malicious or abandoned. Building that apparatus to support
      *zero current external contributors* is exactly the
      pre-engineering §5 calls out.
    - Multiplied release overhead: every Typst bump becomes N+1
      releases instead of one. CI matrices, version compatibility
      tables, "does ferrocv 0.7 work with ferrocv-themes-modern-cv
      0.3?" issues. We would be paying ongoing operational tax for
      a benefit we have not measured.
    - The Universe-cache path (option C) already provides the
      "themes outside this repo" story for free; per-theme repos
      would be a *second* such mechanism, not the first.

**D. Plugin system — runtime-loaded theme crates (rejected as out of
scope).** A theme is a `.so` / `.dylib` / `.dll` the binary loads at
runtime, implementing some `Theme` trait.

  - *Why it was attractive:* maximum extensibility; third parties can
    ship themes without coordinating with us at all.
  - *Why rejected:* fundamentally incompatible with §6 ("themes run
    under Typst's native sandbox, nothing more"). A runtime-loaded
    native code plugin has the full process privilege of the binary —
    it can read `resume.json`, open network sockets, exec anything.
    That is a categorically larger trust surface than the Typst
    sandbox. A constitutional amendment, not a feature, and not
    one we have any reason to want.

**E. Hosted theme registry (rejected as out of scope).** Stand up a
`themes.ferrocv.dev`-style service that indexes contributed themes
with reviews, screenshots, etc. The `themes install` subcommand
queries it.

  - *Why it was attractive:* the discoverability gap option B suffers
    from goes away.
  - *Why rejected:* a hosted service is on the §6 non-goals list,
    and the Typst Universe registry already exists for this exact
    purpose. We do not need to build a second one.

## Consequences

**Positive.**

- The decision matches the code that already shipped — no new
  infrastructure, no migration. The five queued adapters ([#97]–[#101])
  can proceed under unchanged process.
- The user-facing answer to "can I use a theme you don't ship?" is a
  single sentence: `ferrocv themes install @preview/<name>:<version>`.
  No discovery layer to build, no extension API to design.
- The maintainer-facing answer to "should we add this theme?" stays
  qualitative and judgment-based: does it expand visual range, is the
  license compatible, can we afford the vendoring/golden-regen burden.
  Saying *no* is a normal outcome that does not deny the user the
  theme — it just routes them to the cache path.
- §5 is honored: zero pre-engineering, zero distribution apparatus
  built for callers who do not exist.

**Negative.**

- Adapter inclusion remains a judgment call without a written-down
  bar, which means future debates about "should ferrocv ship theme
  X?" recur. That's the cost of qualitative criteria; the alternative
  (a formal inclusion policy) is itself pre-engineering until we have
  enough pressure to need one.
- Bundled adapter sources continue to grow the binary, ~30-50KB per
  adapter. Today this is rounding error; if the adapter set grows
  past the point where the *next* adapter materially changes the
  binary size or the cold-start cost, the revisit-if below kicks in.
- Universe-cache themes are second-class in a real sense: they don't
  get the empty-URL audits, the `@preview/...` removal, or the
  golden regression coverage that bundled themes do. This is the
  honest trade-off — we are saying "we vouch for the bundled set;
  you vouch for what you install." Users surprised by upstream
  behavior on a `themes install`'d theme are on their own, and the
  CLI errors are written with that in mind.

**Non-goals that fall out.**

- No `ferrocv-themes-*` GitHub namespace, crates.io subspace, or
  per-theme release cadence.
- No plugin system, runtime-loaded native theme crates, or shared-
  object theme format.
- No hosted theme registry, search index, or curation service. The
  Typst Universe registry is the discovery surface for non-bundled
  themes.
- No formal theme-inclusion policy beyond the qualitative criteria
  named in *Decision*.

**Revisit if.**

- Bundled theme bytes grow enough that they dominate the binary size
  or measurably affect cold-start. ("Dominate" and "measurably" are
  qualitative on purpose; if either becomes obvious, the next ADR can
  pick numbers from the evidence.)
- A would-be external contributor turns up wanting to ship and
  maintain a theme without going through this repo's review, AND
  enough other contributors want the same thing that one-off
  Universe-cache documentation is not enough. Both conditions — a
  single contributor does not justify standing up a multi-repo
  apparatus.
- Upstream Typst Universe churn on a specific adapter becomes
  frequent enough that bundled-with-ferrocv release cadence
  noticeably lags behind users' expectations. So far, zero data
  points.
- A future ADR weakens §6's "no plugin system" stance for unrelated
  reasons; the distribution question would need to be re-asked under
  the new sandbox model.

## References

- Issue: [#130] — Decide theme distribution model: in-repo vs. per-theme
  repos
- Related issues: [#97], [#98], [#99], [#100], [#101] (queued
  adapter proposals); [#41] (the `themes install` subcommand that
  shipped the Universe-cache path)
- [`CONSTITUTION.md`][const] §4, §5, §6
- [`VENDORING_CHECKLIST.md`][checklist] — the in-repo adapter
  maintenance runbook this ADR commits us to

[#41]: https://github.com/cacack/ferrocv/issues/41
[#97]: https://github.com/cacack/ferrocv/issues/97
[#98]: https://github.com/cacack/ferrocv/issues/98
[#99]: https://github.com/cacack/ferrocv/issues/99
[#100]: https://github.com/cacack/ferrocv/issues/100
[#101]: https://github.com/cacack/ferrocv/issues/101
[#130]: https://github.com/cacack/ferrocv/issues/130
[const]: ../../CONSTITUTION.md
[checklist]: ../../assets/themes/VENDORING_CHECKLIST.md
