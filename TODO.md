# TODO

Planned work is tracked as [GitHub
issues](https://github.com/cacack/ferrocv/issues); this file now holds
only unresolved design questions that don't yet have enough shape to
become actionable issues. Each of these should eventually resolve into
an ADR (committed to the repo) or get folded into a tracking issue's
scope, at which point it gets deleted from here. When this file empties
out, delete it.

## Open questions

- [ ] Confirm Typst HTML export maturity at the time of implementation
      (blocker on the Phase 2 HTML output issue).
- [ ] DOCX strategy — drop entirely, or keep an HTML→pandoc fallback?
      Revisit as part of Phase 2 planning.
- [ ] How should renderer filter flags interact with themes — preprocess
      JSON in Rust before handing to the theme, or pass as theme
      parameters? Probably preprocess (keeps themes simple), but worth
      sanity-checking. Captured in the Phase 5 tracking issue.
- [ ] Library vs. binary split — is there demand for `ferrocv-core` as
      a crate others can embed, or is the CLI sufficient? Revisit once
      Phase 1 lands and we see whether anyone asks.
- [ ] Theme distribution: do we host adapters in this repo, or split
      them into per-theme repos under a `ferrocv-themes-*` namespace?
      Revisit when the third or fourth adapter lands.
- [ ] How to handle JSON Resume `x-` extension fields cleanly across
      adapters that don't know about them (silent drop vs. warn)?
      Needs a decision before the adapter count grows past one or two.
