# Strategic Panel Synthesis — 2026-10-03

## Constitution under review
`ferrocv` renders unmodified JSON Resume v1.0.0 (§1) through in-process Typst (§2) to PDF, HTML, and text as peer targets (§3), with adapter and native themes kept separate (§4), built narrowly (§5), and under hard trust commitments: no network in `render`/`validate`, a single feature-gated `themes install` network entry point, no telemetry, and checksummed (eventually signed) releases (§6). §7 makes **targeted projection** — one master `resume.json` emitting audience-specific cuts, selection in Rust, never in themes, never rewriting — the stated reason the project exists; rendering alone is called a commodity.

## Success Criteria scorecard
0 met · 0 unmet · 0 unmeasurable. **CONSTITUTION.md has no Success Criteria section** — nothing could be scored. Remedy: add Success Criteria and name the command that settles each (or declare it a judgement call) when refreshing via `/panels:constitution`.

## Per-persona verdicts
| Persona | Verdict | Findings (C/H/M/L) |
|---------|---------|--------------------|
| Mission Steward | drifting | 0/2/2/2 |
| Market Strategist | unclear | 0/1/3/2 |
| Roadmap Reviewer | aligned | 0/0/3/2 |
| Audience Advocate | partially-served | 0/2/3/2 |
| Trust Auditor | trustworthy | 0/0/2/2 |

Note: market and trust personas returned reports without writing files; the orchestrator saved them verbatim-in-substance.

## Cross-cutting themes

1. **The differentiator is shipped but under-sold and under-invested** (mission, market, roadmap, audience). Projection shipped in v0.8.0, but README tagline/Why/Goals, Cargo `description`/keywords all pitch rendering — the capability §7 calls a commodity. The only open milestone (v0.10.0) is five theme adapters; projection follow-ups #195/#196 are `priority:low` and unmilestoned.
2. **Two unconnected "audience" concepts** (mission HIGH, market MEDIUM). `classic` and the `themes new` scaffold read `meta.x-audience` to print "Tailored for:", while `tailor --audience` filters on `x-ferrocv.audience` and never sets `meta.x-audience`. Contradicts §7 "themes stay ignorant of audiences" and confuses the headline feature.
3. **Stale surfaces** (all five). CLAUDE.md says v0.6.0 / projection unbuilt; README GitHub Actions example pins v0.4.0 (pre-`tailor`); README Status cites closed "phase milestones"; `themes --help` references "issue #41" as future work.
4. **The getting-started path is weak** (audience HIGH, trust LOW, market LOW). No Install section; prebuilt release binaries omit the `install` feature that the README advertises; README non-goals and integrity caveats (no rewriting/AI/SaaS; checksummed-not-signed; TLS-only `themes install`) live only in the constitution.
5. **Activity has shifted to maintenance** (mission LOW, roadmap MEDIUM). ~32 non-merge commits since v0.9.0, one user-facing. #229's work landed (PR #231, `ferrocv-steward` merges confirmed) but the issue is still open.
6. **Constitution drift** (mission, roadmap). No Success Criteria; no named audience; §5 "Phase 1" framing obsolete; native-theme authoring surface (prelude API, `themes new`, external themes) grew without a §5 exception or ADR.

## Since the last run
No previous run — this is the baseline.

## Alignment gaps
1. **[HIGH] Positioning sells the commodity, not §7** — README tagline/Why/Goals and Cargo metadata omit projection. (§7; market, mission, audience)
2. **[HIGH] Theme reads an audience tag; two unconnected audience vocabularies** — `classic` + scaffold read `meta.x-audience`; `tailor` never sets it. (§7, §4; mission, market)
3. **[HIGH] Mistyped `--audience` silently yields an un-tailored cut** — verified: `tailor --audience securty` exits 0, no stderr. Headline feature fails silently on a high-stakes artifact. (§7; audience)
4. **[HIGH] No end-user install path; release binaries lack `themes install`** — README never says how to get the binary; `@preview` themes need a source rebuild. (§2, §6; audience)
5. **[MEDIUM, cross-flagged] Projection depth unscheduled** — #195/#196 `priority:low`, no milestone, while adapters are `priority:medium`; #196 needs a §7 "select, never generate" guard. (§7; mission, roadmap, audience, market)
6. **[MEDIUM, cross-flagged] Stale docs** — CLAUDE.md, README Actions pin v0.4.0, README Status, `themes --help`. (all)
7. **[MEDIUM] "Stable CLI" claim vs. unflagged default-theme change in v0.9.0**; define stability below 1.0, mark default-output changes breaking. (§6 trust; trust)
8. **[MEDIUM] Release integrity undisclosed** — checksums only, no signing/attestation; README silent; SECURITY.md "no network calls at runtime" over-broad. (§6; trust)
9. **[MEDIUM] v0.10.0 is a catchall with no exit criterion**, repeats v0.9.0's theme; #101 effort:high/priority:low inside it. (§4, §5; roadmap)
10. **[MEDIUM] Theme choice opaque** — `themes list` bare names, no gallery, no format/ATS guidance. (§3; audience)

## Overall alignment
On-mission on the fundamentals: §1, §2, §3 and §6 show no contradicting activity, and §7 projection — the reason to exist — actually shipped. The drift is in emphasis and polish, not direction: the outward story and the forward roadmap both lean on theme breadth (the commodity) while the differentiator has a known completeness hole (#195), a silent-failure mode, and a theme-side audience hook that contradicts §7's own separation rule. Recent energy is almost entirely maintenance. The fix is a re-centering: tell the projection story first, harden it, and schedule its depth ahead of more adapters.

## Constitution suggestions
- Add a **Success Criteria** section with named checks (e.g. "a 2-page cut is produced from `examples/master.resume.json` without hand-editing").
- Name the **audience** (CLI-literate JSON Resume owners maintaining a long master).
- Resolve §7 vs. `meta.x-audience`: either amend §7 to allow Rust to set an opaque display label themes may print, or remove the theme hook.
- Refresh §5's obsolete "Phase 1" framing; record external native-theme authoring as an intentional §5 exception or mark the prelude API unstable.
- Consider moving §6.1's inline amendment history to an ADR.

## Gap ledger
Personas that ran and completed this run: mission, market, roadmap, audience, trust

| Gap | Status | Raised by | First seen | Runs seen |
|-----|--------|-----------|------------|-----------|
| Positioning (README/Cargo) pitches rendering, omits projection | new | market, mission, audience | 2026-10-03 | 1 |
| Theme reads `meta.x-audience`; two unconnected audience vocabularies | new | mission, market | 2026-10-03 | 1 |
| Mistyped `--audience` silently produces un-tailored cut | new | audience | 2026-10-03 | 1 |
| No install path; release binaries lack `install` feature | new | audience, trust | 2026-10-03 | 1 |
| Projection follow-ups (#195/#196) unscheduled, lower priority than adapters | new | mission, roadmap, audience, market | 2026-10-03 | 1 |
| Stale docs (CLAUDE.md, README pins/status, help text) | new | mission, market, roadmap, audience, trust | 2026-10-03 | 1 |
| "Stable CLI" claim vs. unflagged default-output change | new | trust | 2026-10-03 | 1 |
| Release integrity / network-claim wording undisclosed or over-broad | new | trust | 2026-10-03 | 1 |
| v0.10.0 catchall milestone with no exit criterion | new | roadmap | 2026-10-03 | 1 |
| Theme choice opaque (no gallery/format/ATS guidance) | new | audience | 2026-10-03 | 1 |
| README non-goals omit no-rewrite/no-AI/no-SaaS; prior art lacks comparison | new | market | 2026-10-03 | 1 |
| Native-theme authoring surface grew without §5 exception | new | mission | 2026-10-03 | 1 |
| Activity shifted to maintenance; closed work (#229) left open | new | roadmap, mission | 2026-10-03 | 1 |
| Constitution lacks Success Criteria and named audience | new | mission | 2026-10-03 | 1 |
