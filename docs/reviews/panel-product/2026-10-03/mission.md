# Mission Steward Review — 2026-10-03

**Verdict:** drifting

Over the last ~4 months ferrocv delivered what the constitution calls its differentiator: v0.8.0 shipped projection (`tailor`, `--since`, `--max-bullets`, `--redact pii`, curated `--audience` over `x-ferrocv` tags, ADRs 0004/0005), and v0.9.0 shipped the native-theme contract (`classic`, shared prelude, `themes new`). The privacy, offline and embed-Typst commitments (§2, §6) show no contradicting activity: `install` stays a Cargo-feature-gated entry point. The drift is narrow and recoverable. (1) A shipped theme reads an audience tag, which §7 says themes must not do. (2) The next milestone and the project-facing description favor theme breadth over the headline projection feature. (3) Several repo documents still describe a project that predates projection. The last 30 commits are all dependency and CI maintenance, which is not off-mission but means there is no recent mission-serving signal beyond v0.9.x.

## Findings

**[HIGH] Theme reads an audience tag, contradicting "themes stay ignorant of audiences"**
- Constitution section: §7 "Selection lives in Rust, never in themes... themes receive an already-narrowed valid JSON Resume and stay ignorant of audiences and filters." Also §4 (layers separable) and §5 (keep the theme contract simple).
- Observed evidence: `assets/themes/classic/resume.typ` (~l.23-28, 132) renders a "Tailored for: <label>" tagline from `meta.x-audience`. The README Usage section advertises it, and `tests/render_theme.rs:199-246` and `docs/native-themes.md:133` cover it. `assets/scaffold/resume.typ:91` ships the same snippet to every new theme author. `src/` never sets `meta.x-audience`; `tailor --audience security` does not emit it, so the user must hand-set it. Meanwhile `docs/native-themes.md` tells authors the `ferrocv` namespace is reserved so themes never see audience tags.
- Gap: The project now has two unconnected audience vocabularies. `x-ferrocv.audience` is consumed and stripped by Rust. `meta.x-audience` is a free-text label consumed by a theme. The default PDF theme is audience-aware, and the scaffold teaches new theme authors to be audience-aware too. This is the "just-this-one-thing" exception pattern, and the scaffold propagates it.
- Suggested action: Pick one. Either (a) have projection emit `meta.x-audience` from `--audience` (Rust sets it, theme merely displays an opaque string) and amend §7 to say so, or (b) remove the tagline from `classic` and the scaffold. Option (a) fits the "themes are ignorant" intent better only if the amendment is explicit.

**[HIGH] Roadmap and project description lean on the commodity capability, not the stated differentiator**
- Constitution section: §7 Why: "plenty of tools render a JSON Resume to PDF — that capability is a commodity... Maintaining a single source of truth and generating tailored cuts is the differentiating value." Curated selection is "the headline."
- Observed evidence: The only open milestone (v0.10.0) is five theme adapters (#97-#101, none projection-related), repeating the v0.9.0 theme-breadth theme. Projection follow-ups #195 (drop whole entities per audience; the issue says projects can end up as "orphaned title + description with zero bullets") and #196 (collapse old roles) are both `priority:low` and in no milestone; the `feature:projection` label exists but is not driving a milestone. `Cargo.toml` description and keywords ("resume, cv, json-resume, typst, pdf") and the README "Why" and "Goals" list never mention projection or `tailor`; the README opens with "Render JSON Resume to PDF, HTML, and text" (the commodity pitch).
- Gap: The constitution says projection is why the project exists. #195 means curated selection has a known hole in the headline feature, and the next planned work is adapters instead. The public-facing mission statement was not updated when §7 shipped.
- Suggested action: Move #195 (and #196 if wanted) into v0.10.0 or a v0.10-projection milestone ahead of the lower-priority adapters (#101 is effort:high, priority:low). Add projection to the Cargo description, keywords and README Goals. Avoid describing a second tool's feature set as the headline.

**[MEDIUM] Stale mission-bearing docs contradict reality (CLAUDE.md, Cargo.toml)**
- Constitution section: §7 and the constitution's own framing ("what we're building and why"). CLAUDE.md says it defers to the constitution.
- Observed evidence: `CLAUDE.md` says "current release v0.6.0" and that targeted projection "is the next headline feature, tracked under the `v0.8.0` milestone, and not yet built." In fact v0.8.0 shipped on 2026-06-15 (the milestone is closed 7/7) and the latest release is v0.9.1. `Cargo.toml` still reads version 0.9.0 against a v0.9.1 release (release-please lag is plausible and benign).
- Gap: The agent-facing project brief tells contributors the differentiator is unbuilt. That misdirects scope decisions (for example, an agent may treat projection work as speculative under §5).
- Suggested action: Update the CLAUDE.md Project paragraph to current state. Keep it short.

**[MEDIUM] Native-theme authoring surface is growing ahead of a second caller (§5 tension)**
- Constitution section: §5 "We do not pre-engineer extension points, plugin systems, or configuration surfaces for phases we have not started... A second caller is the trigger to generalize." §4 defines the native contract.
- Observed evidence: v0.9.0 added a shared prelude (`assets/themes/_prelude/lib.typ`) with an `ext`/`opt` accessor API (`01a9cbd`), `themes new` scaffolding (`c936925`), `--theme <path>/resume.typ` for user-authored themes, a `golden.txt` stub, and a full authoring guide (`docs/native-themes.md`). #129 asks whether to extract a `ferrocv-core` library crate.
- Gap: Some of this is defensible: the prelude has multiple callers (the three native themes plus `classic`), and §4 names native themes as the long-term contract. But the public prelude API, scaffolder and external theme-author docs commit ferrocv to a stable third-party extension surface, which is what §5 warns against, and no constitution section blesses external theme authoring as an audience. Projection (§7) got ADRs before implementation; the theme-author contract did not.
- Suggested action: Either record this as an intentional §5 exception (a short ADR or constitution note that external native-theme authors are a supported audience) or label the prelude API as unstable in `docs/native-themes.md`. Close or defer #129 explicitly.

**[LOW] "Phase 1" framing and "personal tool" audience in the constitution no longer match the project's shape**
- Constitution section: §5 "Phase 1 is built for Phase 1... this is a personal tool graduating to open-source."
- Observed evidence: Work is now organized as v0.x milestones (v0.8.0, v0.9.0, v0.10.0), not phases; Phase 0-3 milestones are all closed. The project ships a crates.io package, a GitHub Action (`setup-ferrocv`), a forkable example repo (`ferrocv-example`), a themes installer and OpenSSF-badge work (#117), which indicates a public-user audience.
- Gap: §5's rule ("narrower solution when in doubt") still works, but "Phase 1" has no referent, and the constitution does not name an audience beyond the author.
- Suggested action: Refresh §5 wording at the next amendment; see Constitution health.

**[LOW] Recent activity is entirely maintenance; no mission-serving commits in the last 30**
- Constitution section: §6 (reproducible, verifiable releases; trust) and §7 (headline feature).
- Observed evidence: The last 30 commits are Dependabot bumps, an advisory fix (`fix(deps): bump rustls ... RUSTSEC-2026-0285`) and CI automation (`ci: call the shared dependabot-automerge workflow`, #229/#230). The only open work with an unconditional mission link is v0.10.0 adapters. CI automation consumes recent attention but is tooling, which the constitution exempts.
- Gap: Not drift in itself, since dependency hygiene supports §6. It is flagged only because it compounds the roadmap finding above.
- Suggested action: None beyond the roadmap realignment.

## Constitution health

- **No Success Criteria section.** The scorecard reports "No Success Criteria in CONSTITUTION.md." The document says what the project is and refuses, but nothing is measurable (for example, "a user can produce a two-page cut from a 5-page master without hand-editing"). Mission-fit reviews therefore rest on prose and issue labels only.
- **§5 is the vaguest principle.** "Phase 1 is built for Phase 1" refers to phases the project no longer tracks; "when in doubt, pick the narrower solution" is not testable. §5 is the principle the native-theme finding strains against, and there is no way to say definitively whether it is violated.
- **§6.1 is overloaded.** The first two bullets carry multiple amendments inline (cache reader, transitive fetch, `install` feature, TLS-only integrity), making the "hard commitment" hard to scan. It is precise and testable, not vague. Consider moving the amendment history to an ADR and keeping the invariant statement short.
- **§7 versus implementation.** §7 says themes "stay ignorant of audiences," but a shipped theme reads an audience label (see HIGH finding). §7 also doesn't say how audience-tagged content should be surfaced at all; if the tagline is intended, the section should say so.
- **Audience is unnamed.** The constitution never names the target user (JSON Resume owners migrating off the JS toolchain? senior engineers with 5+ page masters?). §7's "5+ pages printed in full" is the only hint. Audience-fit therefore cannot be scored beyond inference.

### Summary counts
critical=0 high=2 medium=2 low=2
