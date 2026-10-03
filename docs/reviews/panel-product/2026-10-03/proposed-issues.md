# Proposed Issues — 2026-10-03

## 1. Fail loudly when `tailor --audience` matches no tags in the master
**Severity:** high  **Persona(s):** audience, foil  **Labels:** bug, feature:projection
**Constitution section:** §7 (curated selection is the headline)

`ferrocv tailor examples/master.resume.json --audience securty` exits 0 with no stderr and writes a plausible-looking, un-tailored cut (only universal content survives). Verified on HEAD. On a document sent to an employer, a silent wrong output is the worst failure mode.

**Approach:** collect the set of audience names present in `x-ferrocv` tags; if `--audience X` is not in it, exit non-zero with `error: audience "securty" matches no tags; known audiences: leadership, security` (mirrors the existing "available themes: …" hint). Optional follow-on: a one-line stderr summary (`kept N entries / M bullets, dropped …`).

**Acceptance criteria**
- [ ] `tailor --audience <unknown>` exits non-zero and stderr lists the known audience names
- [ ] `render --audience <unknown>` (if supported) behaves the same
- [ ] Scenario test in `tests/` covers the unknown-audience case
- [ ] `docs/tailoring.md` Cautions updated

---

## 2. Settle the theme-side `meta.x-audience` hook against §7
**Severity:** high  **Persona(s):** mission, market, foil  **Labels:** enhancement, feature:projection
**Constitution section:** §7 ("themes stay ignorant of audiences"), §4

`classic` (`assets/themes/classic/resume.typ:132`) and the `themes new` scaffold (`assets/scaffold/resume.typ:91`) print "Tailored for: <label>" from `meta.x-audience`, while `tailor --audience` filters on `x-ferrocv.audience` and never sets `meta.x-audience`. Two unconnected audience vocabularies; a theme reads an audience tag the constitution says it must not.

**Approach (recommended):** amend §7 — projection may stamp an opaque display label into `meta.x-audience`; themes may print it, never filter on it. Then have `tailor --audience X` set `meta.x-audience = X` unless the master already sets it. Alternative: remove the tagline from `classic` and the scaffold.

**Acceptance criteria**
- [ ] CONSTITUTION.md §7 states the rule for display labels (amendment PR with rationale)
- [ ] `tailor --audience security` output contains `meta.x-audience: "security"` (unless pre-set)
- [ ] README describes one audience mechanism, not two
- [ ] Golden/scenario test covers the stamped label

---

## 3. Lead README and crate metadata with projection, and add an Install section
**Severity:** high  **Persona(s):** market, mission, audience, trust  **Labels:** documentation
**Constitution section:** §7 (projection is the differentiator), Non-goals, §6

The README tagline/Why/Goals and Cargo `description`/keywords pitch rendering — the capability §7 calls a commodity. Projection first appears ~110 lines in. There is no Install section at all, and release binaries ship without the `install` feature the README advertises.

**Approach**
- Tagline + Cargo `description`: add "maintain one master resume.json, emit audience-specific cuts"; add a `tailor`/projection keyword and Goals bullet.
- Short Install section: release tarballs (with SHA256), `cargo install ferrocv`, `cargo install ferrocv --features install`; state that prebuilt binaries lack `themes install`.
- Mirror the "selects and omits, never rewrites; no AI; no hosted service" non-goals near the projection section.
- Add a one-line "vs" per Prior-art entry.

**Acceptance criteria**
- [ ] README first paragraph and Cargo `description` mention projection/tailored cuts
- [ ] README has an Install section covering all three paths and the `install`-feature caveat
- [ ] README Non-goals include no-rewriting / no-AI / no-hosted-service

---

## 4. Refresh stale status surfaces (CLAUDE.md, README pins/status, `themes --help`)
**Severity:** medium (cross-flagged by all five)  **Persona(s):** all  **Labels:** documentation
**Constitution section:** §7, §6 (trust)

- `CLAUDE.md` says current release v0.6.0 and projection "not yet built" (shipped in v0.8.0; latest v0.9.1).
- README GitHub Actions example pins `setup-ferrocv@v0.4.0` (pre-`tailor`).
- README Status refers to "phase milestones" (all closed).
- `themes --help` still describes `install` as future work for "issue #41".

**Acceptance criteria**
- [ ] None of the four strings above remain; CLAUDE.md drops the hard-coded version so it can't drift again
- [ ] Actions example pin matches the latest release (or uses a documented placeholder convention)

---

## 5. State release integrity and stability accurately below 1.0
**Severity:** medium  **Persona(s):** trust  **Labels:** documentation
**Constitution section:** §6 (reproducible, verifiable releases)

- README says "the CLI surface itself is stable" while v0.9.0 silently changed default PDF output (`classic`) with no breaking marker.
- Releases are SHA256-checksummed only (no signing/attestation); README is silent; `themes install` integrity is TLS-only (constitution only).
- SECURITY.md says "no network calls at runtime" — over-broad given feature-gated `themes install`.

**Approach:** narrow the stability claim ("flags are stable; default themes/appearance may change in 0.x"); use `feat!`/BREAKING CHANGE for default-output changes; one README/SECURITY line on checksummed-not-signed and TLS-only install; reword SECURITY.md network line. Optional: add GitHub artifact attestations to `release.yml` (the "when tooling allows" step).

**Acceptance criteria**
- [ ] README stability statement scoped to 0.x
- [ ] README or SECURITY.md states checksummed-not-signed and TLS-only `themes install`
- [ ] SECURITY.md network statement names `render`/`validate` and the feature-gated exception

---

## 6. Add Success Criteria and a named audience to CONSTITUTION.md
**Severity:** medium  **Persona(s):** mission, foil  **Labels:** documentation
**Constitution section:** whole document (§5 refresh)

The constitution has no Success Criteria (nothing for the panel to score) and never names its audience. §5's "Phase 1" framing is obsolete. Proposed criteria (from the foil): (a) the author's `resume` repo renders every cut from `resume.json` via ferrocv flags alone; (b) an unmatched `--audience` exits non-zero; (c) the README's first paragraph describes projection. Also decide whether external native-theme authoring is an intentional §5 exception.

**Acceptance criteria**
- [ ] `## Success Criteria` section with a named check per criterion (or explicit judgement-call marker)
- [ ] Audience named
- [ ] §5 wording refreshed; native-theme authoring stance recorded
- [ ] Amendment PR explains what changed and why

---

## 7. Help users choose a theme (descriptions, best format, ATS note)
**Severity:** medium  **Persona(s):** audience  **Labels:** enhancement, area:themes
**Constitution section:** §3

`themes list` prints bare names; no gallery, format suitability, or ATS guidance. Users must render all seven to choose.

**Acceptance criteria**
- [ ] README theme table: name, best format, one-line look, ATS note
- [ ] `themes list` (or `--verbose`) shows a one-line description per theme

---

# Triage of existing open issues

| # | Title | Recommendation | Rationale |
|---|-------|----------------|-----------|
| #229 | Adopt shared reusable dependabot-automerge workflow | **Close (completed)** | PR #231 merged 2026-08-09; `BOT_APP_ID`/`BOT_PRIVATE_KEY` set; stub calls shared workflow pinned by SHA; PR #241 (4 workflow files) merged by `app/ferrocv-steward`; push CI runs on merge commits |
| #230 | Auto-merge workflow fails instead of skipping | **Close (completed)** | Local workflow replaced by #231; since 2026-08-09: 63 success, 23 skipped, 1 failure (2026-08-10), none since |
| #196 | Projection: collapse old work entries to one-liners | **Promote → priority:high, v0.10.0, `feature:projection`; edit body** | Add §7 guard (field omission only — keep name/position/dates, drop summary/highlights, never generate text) and the AC "`resume` repo deletes `scripts/gen_curated.py` + `resume-curated.json`" |
| #195 | Projection: drop whole entities per audience | **Promote → priority:medium, v0.10.0, `feature:projection`; edit body** | Remove the retracted "empty-bullet husk" AC (corrected in its own comment); `gen_curated.py` also works around it |
| #97–#100 | Adapters: impressive-impression, moderner-cv, lavandula, cobalt-cv | **Move to new v0.11.0 "Theme breadth" milestone; refresh bodies** | Commodity work per §7; each carries a stale "shape decision pending #102/#103" comment — both closed, so pick vendored vs install-based shape in the body |
| #101 | Adapter: neat-cv | **Remove from milestone (backlog) or close as not planned** | effort:high / priority:low; two `@preview` deps |
| #129 | Decide whether to extract ferrocv-core | **Close (not planned)** | No second consumer after 5+ months; §5 says wait for one; reopen if someone asks |
| #154 | Clear RUSTSEC-2024-0320 | **Keep** | Standing upstream tracker; working as intended |
| #117 | OpenSSF Best Practices badge | **Keep (low)** | Low effort, on-theme for §6 |

**Milestone v0.10.0:** retitle description to "Projection complete — finish §7 for its first real user"; contents #196, #195, new #1, new #2.
