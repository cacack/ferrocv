// Test fixture entrypoint for Stage C cache-hit scenarios. Reads the
// JSON Resume document via the World's `/resume.json` slot and emits
// `basics.name` plus `basics.summary` so the scenario test can assert
// the round-tripped name appears in extracted text. Imports a sibling
// file so the multi-file collection path in `package_cache.rs` is
// exercised end-to-end (not just a single entrypoint file).
#import "./helpers.typ": resume-name, resume-summary

#let r = json("/resume.json")

= #resume-name(r)

#resume-summary(r)
