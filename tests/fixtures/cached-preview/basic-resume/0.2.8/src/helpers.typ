// Sibling helper imported by `lib.typ`. Exists so the cached-package
// resolver collects more than one `.typ` file and so the relative
// `#import "./helpers.typ"` path resolves through the World's virtual
// filesystem under `/themes/preview/basic-resume/0.2.8/src/`.

#let resume-name(resume) = {
  let basics = resume.at("basics", default: (:))
  basics.at("name", default: "")
}

#let resume-summary(resume) = {
  let basics = resume.at("basics", default: (:))
  let summary = basics.at("summary", default: "")
  if summary != "" {
    [#summary]
  }
}
