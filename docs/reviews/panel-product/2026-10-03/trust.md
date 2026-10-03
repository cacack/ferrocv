# Trust Auditor Review — 2026-10-03

(Saved by the orchestrator from the subagent's returned report; the persona could not write files.)

**Verdict:** trustworthy

ferrocv labels itself "Early" and says it is pre-1.0 in both the README and SECURITY.md, then largely delivers what it claims: 10+ tagged releases with a release-please CHANGELOG, SHA256 sidecars for each release binary, SECURITY.md with private reporting and a 72-hour acknowledgement target, active maintenance (RUSTSEC-2026-0285 rustls fix shipped as v0.9.1 on 2026-10-02), experimental HTML export flagged in two places, and a README warning that `tailor` prints PII to stdout by default. The gaps are consistency between surfaces and a few over-broad statements; none would mislead a stranger into harm.

## Findings

**[MEDIUM] "CLI surface itself is stable" sits next to an unflagged behavior-breaking change**
- Claim: README Status says "the CLI surface itself is stable".
- Reality: v0.9.0 changed the default PDF theme to `classic` (CHANGELOG lists it as an ordinary feature, no breaking marker). README discloses it only in the usage body. `themes list` is called a "stable machine-readable contract" with no definition of "stable" below 1.0.
- Trust cost: someone pinning on "stable" and upgrading 0.8 → 0.9 silently gets different PDFs.
- Action: narrow the claim ("CLI flags are stable; default themes and output appearance may change between 0.x releases"); use `feat!` / BREAKING CHANGE for default-output changes.

**[MEDIUM] Release integrity: checksums only, no signing or provenance, and the README does not say so**
- Constitution §6: "release artifacts are checksummed, and (when tooling allows) signed."
- Reality: `.github/workflows/release.yml` uses taiki-e/upload-rust-binary-action with `checksum: sha256` only; no attestation/provenance. SECURITY.md lists unsigned artifacts as in scope. The §6 "TLS only" integrity caveat for `themes install` is absent from README.
- Trust cost: same-origin `.sha256` protects against corruption, not tampering; the "Trust is a feature" framing may imply signed releases.
- Action: one README/SECURITY line stating checksummed-not-signed and TLS-only for `themes install`; or add GitHub artifact attestations (the "tooling allows" step).

**[LOW] CLAUDE.md contradicts the README and shipped releases**
- CLAUDE.md says "current release v0.6.0" and projection "not yet built"; projection shipped in 0.8.0, latest is v0.9.1.
- Action: update, or drop the version/status line so it cannot drift.

**[LOW] SECURITY.md's "no network calls at runtime" is broader than the real surface**
- `themes install` makes network calls (feature-gated `install`). README and constitution are accurate; SECURITY.md drops the qualifier.
- Action: reword to "`render` and `validate` make no network calls; the optional, feature-gated `themes install` is the only network-capable command", and state whether prebuilt binaries include `install` (release.yml suggests not).

## Notes
- Local checkout was 16 commits behind origin/main (0.9.1 release commit not pulled); snapshot version metadata (0.9.0) is stale for that reason.
- No CONTRIBUTING.md / CODE_OF_CONDUCT.md — not a trust gap for a personal "Early" project.
- Open adapter issues (#97–#101) and projection extensions (#195, #196) are not promised in README.
- RUSTSEC-2024-0320 tracked openly in #154 — positive transparency signal.
- No Success Criteria in CONSTITUTION.md; nothing scored.

### Summary counts
critical=0 high=0 medium=2 low=2
