# 0001. Drop DOCX as an output format

**Status:** Accepted
**Date:** 2026-04-19

## Context

Phase 2 of `ferrocv` ([#14]) adds multi-format output: HTML ([#44]),
plain text, and a decision on DOCX. DOCX is attractive because some
consumers of resumes — legacy ATS pipelines, recruiters forwarding
documents, certain enterprise HR workflows — still expect a `.docx`
file. JSON Resume's original tooling does not produce DOCX directly,
but the gap is real enough that users occasionally ask for it.

The relevant constraints, all from [`CONSTITUTION.md`](../../CONSTITUTION.md):

- **§2 Embed Typst; never subprocess it.** The rule is specific to
  Typst, but its reasoning — that a subprocess invalidates the
  "single static binary" premise — generalizes to any external
  renderer.
- **§3 Multi-format output is first-class.** PDF, HTML, and plain
  text are parallel targets. DOCX was *not* named as a first-class
  target in §3.
- **§6 Trust is a feature.** The pitch is "single static binary, no
  Node, no TeX." Adding a runtime dependency erodes that pitch even
  if it's gated behind a flag.

Issue [#46] framed the decision as: drop DOCX entirely, or ship an
opt-in HTML → `pandoc` → DOCX fallback gated by `--format docx`,
erroring if `pandoc` is not on `PATH`.

## Decision

**Drop DOCX as an output format.** `ferrocv` will not emit `.docx`,
will not ship a `--format docx` flag, and will not invoke `pandoc` or
any other external converter. Users who need DOCX can pipe our HTML
output through `pandoc` themselves — that workflow is a user choice,
not a `ferrocv` feature.

## Alternatives considered

**A. Drop DOCX (chosen).** Emit PDF, HTML, and plain text. If a user
needs DOCX, they run `pandoc resume.html -o resume.docx`. Pandoc's
presence and version are their problem, not ours.

**B. Opt-in HTML → pandoc pipeline.** Ship `--format docx`; at
invocation time, look up `pandoc` on `PATH` and error clearly if
missing. Pandoc is never bundled, never installed on the user's
behalf.

  - *Why it was attractive:* DOCX support with zero bundled
    dependency; "you have pandoc → it works, you don't → clean
    error" is a defensible contract.
  - *Why rejected:* the flag itself is the problem. A supported
    `--format docx` creates the expectation that `ferrocv` renders
    DOCX, which obligates us to test it, pin a pandoc version range,
    document its failure modes, handle pandoc's CSS/font quirks that
    differ from what our HTML output targets, and field bug reports
    when users' pandoc installs disagree with ours. That is
    materially more surface area — and a materially different trust
    story — than "we emit HTML." Dropping it costs one `pandoc`
    invocation for the users who actually want DOCX; keeping it
    costs ongoing maintenance and a weakened "no external runtime"
    claim forever.

**C. Native DOCX writer in Rust.** Use a crate like `docx-rs` (or
write one) to emit DOCX from parsed JSON Resume data directly, no
subprocess.

  - *Why it was attractive:* no external runtime, no erosion of §6.
  - *Why rejected:* DOCX is OOXML — a large, under-specified, Office-
    quirked format. A credible writer is a project on the scale of
    `ferrocv` itself, for an output format that is not in our
    first-class set. Out of scope, and [#46] named it as a
    non-goal.

**D. Cloud / LibreOffice subprocess.** Both are excluded up front:
cloud conversion violates §6's "resume data never leaves the
process," and LibreOffice is the same subprocess-runtime problem as
pandoc with a much heavier footprint.

## Consequences

**Positive.**

- The "single static binary, no Node, no TeX, no external runtime"
  pitch remains literally true, not "true with a footnote."
- No pandoc version matrix, no CI job installing pandoc, no
  documentation for pandoc edge cases, no bug reports wedged between
  our HTML and pandoc's DOCX interpretation of it.
- The contract for users who need DOCX is clearer than an opt-in flag
  would be: "we emit HTML; use pandoc if you need DOCX." They own
  the pandoc install and the pandoc version.

**Negative.**

- Users whose workflow genuinely requires DOCX (certain ATS uploads,
  recruiter hand-offs) cannot get it from `ferrocv` alone. They pay
  the cost of a second tool. This is the trade we are making.
- If a DOCX-consuming ATS ecosystem becomes common enough that users
  start pairing `ferrocv` with `pandoc` as a matter of course, the
  case for revisiting this decision will strengthen. That is a
  future ADR's problem, not this one's.

**Non-goals that fall out.**

- No `--format docx` flag, now or via feature flag.
- No pandoc (or any other renderer) on `PATH` at runtime.
- No native Rust DOCX writer.
- No cloud / SaaS / LibreOffice conversion path.

**Revisit if.**

- A native Rust DOCX writer of sufficient quality emerges as a
  maintained dependency (not a vendored fork), *and* user demand
  justifies the added scope. Both conditions — not either.

## References

- Issue: [#46] — DOCX strategy ADR
- Related: [#14] Phase 2 tracking, [#44] HTML output
- `CONSTITUTION.md` §2, §3, §6

[#14]: https://github.com/cacack/ferrocv/issues/14
[#44]: https://github.com/cacack/ferrocv/issues/44
[#46]: https://github.com/cacack/ferrocv/issues/46
