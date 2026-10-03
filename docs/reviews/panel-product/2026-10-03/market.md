# Market Strategist Review — 2026-10-03

(Saved by the orchestrator from the subagent's returned report; the persona could not write files.)

**Verdict:** unclear

**Market scale:** public-OSS, early stage — personal tool graduating to open source (§5). Published on crates.io, a GitHub Action, and a forkable `ferrocv-example` starter; no evidence of external users in the snapshot.

The category is clear (Rust CLI rendering JSON Resume to PDF/HTML/text, "single static binary, no Node or TeX"). The problem: the first screen sells the capability §7 calls a commodity. The stated differentiator — targeted projection — has shipped (`tailor`, `--audience`, `--since`, `--max-bullets`, `--redact`) but appears only deep in Usage. A stranger skimming tagline/Status/Why/Goals sees "another JSON Resume renderer, in Rust".

## Findings

**[HIGH] The headline differentiator is missing from every positioning surface**
- Constitution: §7 — "Maintaining a single source of truth and generating tailored cuts per application is the differentiating value."
- Evidence: README tagline and Cargo `description` mention only rendering; keywords `resume, cv, json-resume, typst, pdf`; README "Why" argues only about the fragile JS theme ecosystem; "Goals" has no projection bullet; "Status" mentions only themes; projection first appears ~110 lines in.
- Action: add "…maintain one master resume.json and emit audience-specific cuts" to tagline and Cargo `description`; add a Goals bullet; consider a `tailor` keyword.

**[MEDIUM] "Prior art" lists alternatives but never says how ferrocv differs**
- Evidence: four one-line entries (`typst-jsonresume-cv`, Typst Universe, `jsonresume-renderer`, `typst.ts`) with no comparison; the closest peer (`jsonresume-renderer`, also Rust) is described only as "not Typst"; the JS `resumed` + headless-browser flow being replaced is never named.
- Action: per-entry "vs" clause or a 3-row comparison stating offline, single binary, projection.

**[MEDIUM] Two "audience" mechanisms in the README blur the headline feature**
- Constitution: §1, §7.
- Evidence: `classic` prints "Tailored for: <label>" from `meta.x-audience`; `--audience` filters on `x-ferrocv.audience`. Different keys, locations, and no stated relationship.
- Action: one sentence stating the relationship, or move the `meta.x-audience` note into the projection section.

**[MEDIUM] README non-goals omit the two that matter most for positioning**
- Constitution: Non-goals (no hosted service; no authoring/rewriting), §6 (no LLM calls).
- Evidence: README lists only schema, input formats, general Typst build tool.
- Action: mirror "selects and omits, never rewrites; local only; no AI; no hosted service" near the projection section — a real contrast with AI-rewrite tailoring tools.

**[LOW] Roadmap emphasis and stated Goals lean on the commodity (theme breadth)**
- Evidence: all five v0.10.0 issues are adapters (#97–#101); #195/#196 `priority:low`, no milestone; Goals stress "visual variety from day one".
- Action: none beyond finding 1; if projection stays low priority the README should still lead with it.

**[LOW] First screen is badge-heavy; maturity signals stale**
- Evidence: eleven badges above the value prop; GitHub Actions example pins `setup-ferrocv@v0.4.0` while latest is v0.9.1; CLAUDE.md still says v0.6.0.
- Action: reorder so the tagline leads; update the example pin.

## Notes
- Unstated category opportunity: "resume-as-code / CI-published resume" (CLI + Action + example template).
- Implicit audience: people with an existing `resume.json` unhappy with JS themes; a one-line "Who is this for" would help.
- No Success Criteria to score.

### Summary counts
critical=0 high=1 medium=3 low=2
