// Stage C regression fixture: this entrypoint deliberately tries to
// import another `@preview/...` package. Even though the SURROUNDING
// theme was resolved out of the offline cache, the inline import must
// still be rejected at compile time by `FerrocvWorld::source` /
// `file` (CONSTITUTION §6.1). The scenario test in
// `tests/render_preview_cli.rs` asserts the rejection.

#import "@preview/cetz:0.2.0": *

#let r = json("/resume.json")

= #r.at("basics", default: (:)).at("name", default: "")
