# Audience Advocate Review — 2026-10-03

**Verdict:** partially-served

**Stated audience (from CONSTITUTION.md):** The constitution has no explicit audience section. The audience is inferred from §1, §3, §7, the non-goals and the README. It is people who already keep, or can keep, a JSON Resume `resume.json`. They are comfortable on a command line and want a single-binary renderer with no Node or TeX (§2). They want PDF, HTML and ATS-friendly text output (§3). Above all they want one master resume with audience-specific cuts (§7). §5 says it is "a personal tool graduating to open-source". I judged CLI-literate individual job-seekers, not Rust developers.

I walked this audience through the surfaces: README, `docs/tailoring.md`, `examples/master.resume.json`, and live runs of the debug binary built at HEAD (it reports 0.9.0, though the latest release is v0.9.1). The core value moment works well. The tailoring guide is excellent: it has a worked table, an honest cautions section and a ready-made example master. Validation and theme errors are readable, with useful hints such as "available themes: …". The gaps are before the value moment (how to get the binary, and what the binary can do) and inside it (silent failures in the headline feature). None of these blocks a determined user, but several would bounce a casual one.

## Findings

**[HIGH] No end-user install path in the README, and the release binary lacks a README-advertised feature**
- Constitution audience: "single static binary, no Node, no TeX" (§2); users who "bring their existing `resume.json`" (§1).
- Observed evidence:
  - A grep of README.md finds no "Install" section. It has no `cargo install ferrocv` line and no pointer to release downloads or the checksummed tarballs. The only binary-acquisition instruction is the GitHub Actions `setup-ferrocv` snippet. The README's Usage section starts with `ferrocv validate resume.json` and assumes the binary exists.
  - `.github/workflows/release.yml` builds release binaries via `taiki-e/upload-rust-binary-action` with no `features:` input. `Cargo.toml` sets `default = []`, so the `install` feature is off in release binaries.
  - The README documents `themes install` and `--theme @preview/...` as ordinary features. The caveat appears only deep in the "theme resolution" prose: "Builds without the `install` feature reject `@preview/...` specs ... rebuild with `--features install`".
- Audience cost: A resume author who downloads the release binary cannot use Typst Universe themes. They are told to rebuild from source with a Rust toolchain, which defeats the "no toolchain" pitch. A first-time reader also cannot tell how to get the tool at all, except through the `ferrocv-example` fork path.
- Suggested action: Add a short "Install" section covering release tarballs, `cargo install ferrocv`, and `cargo install ferrocv --features install`. State up front that prebuilt binaries ship without `themes install`. Alternatively, ship a separate `-install` release asset.

**[HIGH] A mistyped `--audience` value silently produces a stripped, un-tailored resume**
- Constitution audience: §7 "Curated selection is the headline", with the promise that the user keeps one master and nothing drifts.
- Observed evidence: I ran `ferrocv tailor examples/master.resume.json --audience securty -o a.json`. It exited 0 with no stderr output and wrote a document. Because untagged content is universal and anything tagged for another audience is dropped, the cut contained only the universal content. The tool does not check the value against the audience tags actually present in the master. `docs/tailoring.md` "Cautions" documents the sibling failure of mistyped `x-` namespaces, "Typos in the namespace fail silently ... the resulting cut looks plausible but is actually un-tailored". That caution is the authors' acknowledgement of the hazard, not a mitigation.
- Audience cost: The person most likely to hit this is the one who sends the cut to a recruiter. A wrong flag value or tag spelling yields a plausible but wrong PDF with no signal. This is a high-stakes artifact.
- Suggested action: When `--audience X` matches zero tags anywhere in the master, emit a stderr warning. A fuller version would list the known audience names, in the style of the existing "available themes: …" hint. Keep exit 0 if desired. Printing a one-line "kept N / dropped M entries and K bullets" summary would also make the cut auditable.

**[MEDIUM] The README is not organized around the first-run journey and has drifted in places**
- Constitution audience: the user "brings their existing `resume.json`" (§1) and wants value fast.
- Observed evidence:
  - The README has about 300 lines of mixed reference, theme-resolution internals, `themes install` cache details, and a GitHub Actions section before reaching Contributing and Development.
  - The README's Usage block shows `--since`, `--max-bullets` and `--redact`, but never the headline `--audience` flag. It appears only further down, in the Projection section, and the full how-to is in `docs/tailoring.md`.
  - The GitHub Actions example pins `setup-ferrocv@v0.4.0` and `version: v0.4.0`, but the latest release is v0.9.1. Copy-pasting it gives users a version that predates `tailor`, `classic` and projection. The README itself says to keep the two pins matched.
- Audience cost: A reader looking for "make me a tailored security resume" must scan the whole page. A user who copies the CI snippet runs a four-minor-versions-old binary without the feature they came for.
- Suggested action: Put a 3-step quickstart at the top (install, validate, `render --audience`). Bump the example pin, or use a `latest`/placeholder convention. Move the cache and install internals to a docs page.

**[MEDIUM] No guidance for getting an existing resume into a good master, and no way to preview cut size**
- Constitution audience: §7 "one comprehensive master ... 5+ pages printed in full" cut into "a focused two-page document". Non-goal: "does not auto-fit content to a page count."
- Observed evidence: The user's real task is a 2-page cut, but nothing reports page count, and no workflow covers it. The tailoring guide covers tagging and filters but not an iterate loop. It has no "render, check the page count, tighten `--since`/`--max-bullets`" advice. It also has no `--dry-run`/summary output and no guidance on tag-naming conventions. Tagging is hand-edited JSON with index-parallel arrays that the guide itself warns can silently misalign after a reorder (Cautions). Open issues #195 (drop whole entities per audience) and #196 (collapse old roles to one-liners) are the natural next steps, both at priority:low. Collapsing old roles is the most common way people actually shrink a long resume, and `--since` drops them entirely.
- Audience cost: The target user has to trial-and-error cut sizes by opening PDFs. Hand-editing positional tag arrays on large masters is error-prone, which is exactly the situation the guide warns about.
- Suggested action: Document a "tag, cut, check length, adjust" loop in `docs/tailoring.md`. Consider a stderr one-line summary (entries and bullets kept or dropped) on `tailor`. Reconsider #196's priority given that it matches how audience members actually shrink a resume.

**[MEDIUM] Theme choice is hard for a non-developer: no gallery, no previews, and jargon names**
- Constitution audience: "visual variety from day one" (README Goals).
- Observed evidence: `ferrocv themes list` prints seven bare names (`basic-resume`, `classic`, `fantastic-cv`, `html-minimal`, `modern-cv`, `text-minimal`, `typst-jsonresume-cv`) with no description, intended format or screenshot. Several are named after upstream Typst templates, not looks. The README never says which are PDF-suited or HTML-suited. The documented default switch ("PDF default became `classic` in v0.9; if you relied on the older extraction-tuned PDF, pass `--theme text-minimal`") shows that users cannot tell which output is ATS-safe versus pretty. There is also no sample output in the repo. `dist/resume.pdf` is a gitignored local artifact.
- Audience cost: Choosing a theme means rendering all seven and opening each file. The ATS-friendliness question behind §3 is unanswered at the point of choice.
- Suggested action: Add a theme table to the README (name, best format, one-line look, ATS note), and ideally a screenshot of each theme from the example master. A one-line description in `themes list --verbose` would be a small step.

**[LOW] Help surfaces for stuck users are thin**
- Observed evidence: No `.github/ISSUE_TEMPLATE/`, no FAQ, no discussions pointer in the README, and no CONTRIBUTING.md (per the snapshot). The README's only help link is the Issues tab. The README says "Early" yet makes the CLI stable.
- Audience cost: A user whose render fails, or whose cut looks wrong, has no template that asks for the version, command and sample JSON. Maintainer effort is low and for a personal tool this is appropriate, so I rate it LOW.
- Suggested action: Add a bug-report issue template and a short "Troubleshooting" section: exit codes, the `x-ferrocv` typo, no system-font scan, and the offline `@preview` hint.

**[LOW] Error text is generally good; a few messages leak maintainer context**
- Observed evidence:
  - Good: `error: schema validation failed (1 error); no output written` followed by `/basics/email: "bad" is not a "email"`. Also the unknown-theme hint listing available themes, and the JSON syntax error showing line and column.
  - Weaker: the `themes --help` text refers to "issue #41" and describes design intent ("leaves room for a sibling `themes install` ... when issue #41 adds ...") although `install` already exists in the same command list. This is stale, internal-facing help text.
  - Weaker: schema paths print as JSON pointers (`/basics/email`) with no note on which field to fix, which is fine for developers but slightly terse for non-developers.
- Suggested action: Refresh the `themes` long-help to drop the issue reference. Optionally add the offending value's expected format to common validation errors.

## Notes
- All of the above takes the CLI-literate resume author as the audience. If the audience is narrowed to "Chris, personal use", most findings fall to LOW, but §5 explicitly says the tool is "graduating to open-source", so I applied the wider lens.
- Strengths worth keeping: `docs/tailoring.md` is exemplary. It uses a worked example, calls out failure modes, and documents the "untagged means universal" default that makes adoption incremental. `examples/master.resume.json` gives a concrete starting point. Exit codes and stdout/stderr discipline are documented, and PII-to-stdout is called out. The `ferrocv-example` fork-and-push template is a strong value-moment shortcut, but it is mentioned only after the Usage block.
- Out of scope here (other personas): theme-market positioning, trust claims (§6), contributor setup.

### Summary counts
critical=0 high=2 medium=3 low=2
