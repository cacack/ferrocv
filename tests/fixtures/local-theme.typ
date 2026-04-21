// Minimal Typst source used by the local-path theme scenario tests
// in `tests/render_cli.rs`. Reads the JSON Resume from `/resume.json`
// (the virtual path every `FerrocvWorld` exposes) and emits
// `basics.name` as the document's single heading so tests can assert
// the value round-trips end-to-end.
//
// Deliberately tiny and dependency-free — no imports, no package
// references — so it also serves as the narrowest possible exercise
// of `resolve_theme`'s local-path branch.

#let resume = json("/resume.json")
#let name = if type(resume) == dictionary and "basics" in resume {
  let b = resume.at("basics")
  if type(b) == dictionary and "name" in b { b.at("name") } else { "" }
} else { "" }

= #name
