# Roadmap Reviewer — 2026-10-03

**Verdict:** aligned

The roadmap respects the constitution. I found no open issue or recent commit that drives the project toward a stated non-goal. The headline principle (§7, targeted projection) shipped under the v0.8.0 milestone (closed 7/7, released 2026-06-15). The tailor and `--audience` flows are documented in the README and in `docs/tailoring.md`. The one open milestone, v0.10.0, holds five Typst Universe adapter issues, which map to §4 (adapters). The Success Criteria scorecard reports "No Success Criteria in CONSTITUTION.md — nothing to score", so I anchored on the principles and non-goals instead. The weaknesses are about visibility and energy, not direction.
- Since v0.9.0 (2026-06-26) there have been 32 non-merge commits. Only one is a user-facing change (`fix(classic): keep entries together across page breaks`, 2026-07-05). The rest are CI, dependency or advisory maintenance.
- v0.10.0 has had zero closed issues.
- There is no discoverable forward plan beyond a milestone with five adapter issues.
- CLAUDE.md is stale about the very feature the constitution calls differentiating.

Forge data (issues, milestones) was available in the snapshot. I did not read issue bodies, only titles, labels and milestones.

## Findings

**[MEDIUM] Recent resource spend is almost entirely maintenance; stated focus area is idle**
- Constitution section: §7 names projection as "the differentiating value"; §4 / the v0.10.0 milestone ("Theme breadth") is the stated next theme work.
- Observed evidence: `git log --since=2026-06-26 --no-merges` shows 32 commits. The non-dependabot ones are almost all `ci:` (merge-queue triggers, shared dependabot-automerge workflow #229/#231, SHA pinning, CodeQL bump) or advisory fixes (`fix(deps): bump rustls ... RUSTSEC-2026-0285`, `fix(deps): clear RustSec advisories blocking CI`). The last 30 commit subjects are all dependabot, CI or advisory work. Feature and fix commits in the roadmap's focus areas: one (`fix(classic)`, 2026-07-05). The v0.9.1 release (2026-10-02) follows a roughly 3.5-month gap in feature work. Releases ran monthly or faster from April to June 2026.
- Gap: effort has shifted from product (v0.2 to v0.9 in about 10 weeks) to hygiene. That is reasonable after a burst, but the five v0.10.0 adapter issues (#97 to #101) have not moved, and neither have the projection follow-ups (#195, #196).
- Suggested action: either schedule the v0.10.0 work, starting with the lowest-effort adapter #97 (impressive-impression, effort:low), or record the pause explicitly. Move the CI-only issues #229 and #230 to done or close them. #229's commits have already landed.

**[MEDIUM] Projection follow-ups are unmilestoned and low priority although projection is the headline**
- Constitution section: §7 "Curated selection is the headline" and "plenty of tools render a JSON Resume to PDF... [projection] is the differentiating value".
- Observed evidence: #195 (drop whole entities per audience, project-level filtering) and #196 (collapse old work entries to one-liners) are both labeled `feature:projection`-adjacent enhancements with `priority:low` and no milestone. By contrast, theme adapters #98 to #100 and #97 are `priority:medium` and sit in v0.10.0. #196 sits close to the non-goal line "does not generate, summarize, or reword bullets". Collapsing to a one-liner is only acceptable if it selects existing fields (for example the title and dates) and does not synthesize text.
- Gap: the open work gives the commodity capability (more themes) a higher priority than the differentiating one (fuller projection). The completeness gap in curated selection (`#195`: `project` and other entities lack per-audience dropping, while `work` and `highlights` have it) has no scheduled home. #196 needs a non-goal check before work starts.
- Suggested action: put #195 in a milestone (a v0.10.x or v0.11.0 "Projection depth" milestone). Add an acceptance note to #196 saying it must use only existing fields (selection, not generation) per §7. If it cannot meet that bar, close it as wontfix or take it to an amendment PR.

**[MEDIUM] v0.10.0 milestone is a weakly scoped catchall that repeats v0.9.0**
- Constitution section: §4 (adapters give "visual variety on day one") and §5 ("Simple now; iterate later... A second caller is the trigger to generalize").
- Observed evidence: v0.10.0 is described as "Theme breadth — additional Typst Universe adapters built on top of the native theme contract". v0.9.0 had the near-identical title and description. v0.10.0 has open=5, closed=0, no due date. Every issue is a one-per-template adapter (#97 impressive-impression, #98 moderner-cv, #99 lavandula, #100 cobalt-cv, #101 neat-cv). The efforts are mixed (#101 is effort:high, #97 effort:low).
- Gap: the milestone has no completion criterion beyond "all five adapters". It could sit open indefinitely and gate a release on low-value work. #101 is effort:high with priority:low and is in the same milestone as medium-priority items. The milestone is a phantom risk: it exists, but nothing is moving.
- Suggested action: split it. Keep the milestone to the medium-priority adapters, or ship adapters incrementally in patch releases. Move #101 (effort:high, priority:low) out of the milestone. Give the milestone a stated reason to exist (for example "N adapters by date X").

**[LOW] CLAUDE.md (and README Status) understate shipped projection and misstate the release**
- Constitution section: §7 and the roadmap-clarity expectation that a stranger can tell what is coming.
- Observed evidence: `/Users/chris/devel/home/ferrocv/CLAUDE.md` line 13 says "current release v0.6.0" and line 16 says projection is "tracked under the `v0.8.0` milestone, and not yet built". In fact v0.8.0 shipped 2026-06-15 and the current release is v0.9.1. The README documents `ferrocv tailor`, `--audience`, `--since`, `--max-bullets` and `--redact`. The README Status says "Early... Additional themes and native-theme tooling are tracked as GitHub issues and organized into phase milestones". The phase milestones are all closed and the live milestones are version-numbered.
- Gap: planning docs lag reality. An agent or contributor reading CLAUDE.md will think the headline feature is unbuilt.
- Suggested action: update the Project paragraph in CLAUDE.md (release number, projection shipped, and remaining projection work as #195 and #196). Refresh the README Status wording.

**[LOW] No discoverable roadmap beyond the milestone list**
- Constitution section: the constitution defers "how" to GitHub issues; there is no stated roadmap file.
- Observed evidence: ROADMAP.md is absent. CONTRIBUTING.md is absent. Six of seven milestones are closed. v0.10.0 is the only forward signal. Open issues that are not adapters are CI/chore (#229, #230, #154, #117) or open questions (#129 on extracting a `ferrocv-core` crate).
- Gap: a visitor cannot tell what comes after the five adapters. #129 (a library-crate extraction question, effort:medium) is the one item that touches §5, since generalizing before a second caller exists would conflict with it. It is correctly parked as `question` / `priority:low`. Keep it parked until a real second consumer appears.
- Suggested action: either add a short ROADMAP.md (next: adapters, projection depth, decision on #129) or keep a pinned tracking issue (the repo already has a `tracking` label). At this scale (v0.9.x, one maintainer) a milestone plus a tracking issue is enough.

## Non-goal discipline check

- Schema extension, other input formats, general-purpose Typst build tool, hosted service or web UI, and content authoring: no open issue or recent commit touches these. The `x-ferrocv` tagging and the `meta.x-audience` approach use the §1 `x-` mechanism. #196 is the only item to watch (see the MEDIUM finding above).
- Trust (§6): recent activity strengthens it (rustls advisory fix, SHA-pinned workflows, CodeQL, OpenSSF Scorecard). #117 (Best Practices badge) is on-theme.

## Roadmap visibility

Partial. Milestones are used and kept tidy: the closed ones carry descriptions that point at constitution sections, and the one open milestone is v0.10.0. There is no ROADMAP.md, no CONTRIBUTING.md, and no due dates. The README says phase milestones organize the work, but those are all closed. The plan after v0.10.0 is implicit. For a project of this scale that is acceptable, but projection depth (§7's headline) is not visible on any milestone.

### Summary counts
critical=0 high=0 medium=3 low=2
