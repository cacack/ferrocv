// ferrocv glue entrypoint for the vendored `fantastic-cv` theme.
//
// This file is authored by ferrocv (NOT vendored). It imports the
// verbatim upstream `fantastic-cv.typ` builders and adapts JSON Resume
// v1.0.0 data to the argument shapes fantastic-cv expects. The adapter
// layer lives here so the vendored file stays byte-for-byte upstream —
// re-vendoring is a mechanical copy. See VENDORING.md.
//
// CONSTITUTION §1: JSON Resume is the canonical input; every field is
// optional per the v1.0.0 schema. Every read uses `.at(..., default: ...)`.
// CONSTITUTION §5: no section-toggle knobs; every section with data renders.

#import "fantastic-cv.typ": config, render-basic-info, render-education, render-work, render-project, render-volunteer, render-award, render-certificate, render-publication, render-custom

// The FerrocvWorld serves the JSON Resume bytes at the virtual path
// "/resume.json". Same convention as every other ferrocv theme.
#let r = json("/resume.json")

// Optional-field shim for `basics` — absent, null, or partially-filled
// sub-objects must all render without a dict-lookup panic.
#let basics = r.at("basics", default: (:))
#let location = basics.at("location", default: (:))
#let city = location.at("city", default: "")
#let region = location.at("region", default: "")
#let address = if city != "" and region != "" {
  city + ", " + region
} else if city != "" {
  city
} else {
  region
}
#let basics_name = basics.at("name", default: "")
#let basics_email = basics.at("email", default: "")
#let basics_phone = basics.at("phone", default: "")
#let basics_url = basics.at("url", default: "")

// Profiles: fantastic-cv's render-basic-info reads `profile.network`,
// `profile.url`, and `profile.username` on every profile. Project each
// JSON Resume profile entry so those three fields always exist.
#let basics_profiles = basics.at("profiles", default: ()).map(p => (
  network: p.at("network", default: ""),
  url: p.at("url", default: ""),
  username: p.at("username", default: ""),
))

// Apply fantastic-cv's page/text config with its default knobs.
// CONSTITUTION §5: no user-facing configuration yet — a second caller
// is the trigger to generalize.
#show: config.with()

#render-basic-info(
  name: basics_name,
  location: address,
  phone: basics_phone,
  email: basics_email,
  url: basics_url,
  profiles: basics_profiles,
)

// Section: work. JSON Resume key is singular `work`; fantastic-cv
// expects an argument named `works`. Per-entry: JSON Resume `summary`
// → fantastic-cv `description`. JSON Resume has no work-level
// `location`; default to empty.
#if "work" in r and r.work != none and r.work.len() > 0 {
  render-work(r.work.map(w => (
    name: w.at("name", default: ""),
    location: w.at("location", default: ""),
    url: w.at("url", default: ""),
    // ferrocv glue: JSON Resume `summary` → fantastic-cv `description`.
    description: w.at("summary", default: ""),
    position: w.at("position", default: ""),
    startDate: w.at("startDate", default: ""),
    endDate: w.at("endDate", default: ""),
    highlights: w.at("highlights", default: ()),
  )))
}

// Section: education. JSON Resume has no education-level `location`;
// default to empty. fantastic-cv `educations` (plural) ← JSON Resume
// `education` (singular).
#if "education" in r and r.education != none and r.education.len() > 0 {
  render-education(r.education.map(e => (
    institution: e.at("institution", default: ""),
    location: e.at("location", default: ""),
    url: e.at("url", default: ""),
    area: e.at("area", default: ""),
    studyType: e.at("studyType", default: ""),
    startDate: e.at("startDate", default: ""),
    endDate: e.at("endDate", default: ""),
    score: e.at("score", default: ""),
    courses: e.at("courses", default: ()),
  )))
}

// Section: projects. fantastic-cv reads `source_code` (no JSON Resume
// equivalent; leave empty) and `roles` (JSON Resume schema has `roles`
// and also `keywords`; prefer `roles`, fall back to `keywords`).
#if "projects" in r and r.projects != none and r.projects.len() > 0 {
  render-project(r.projects.map(p => (
    name: p.at("name", default: ""),
    url: p.at("url", default: ""),
    source_code: "",
    roles: p.at("roles", default: p.at("keywords", default: ())),
    startDate: p.at("startDate", default: ""),
    endDate: p.at("endDate", default: ""),
    description: p.at("description", default: ""),
    highlights: p.at("highlights", default: ()),
  )))
}

// Section: volunteer. JSON Resume key is singular `volunteer`;
// fantastic-cv expects `volunteers`. JSON Resume has no volunteer-level
// `location`; default to empty.
#if "volunteer" in r and r.volunteer != none and r.volunteer.len() > 0 {
  render-volunteer(r.volunteer.map(v => (
    organization: v.at("organization", default: ""),
    position: v.at("position", default: ""),
    url: v.at("url", default: ""),
    startDate: v.at("startDate", default: ""),
    endDate: v.at("endDate", default: ""),
    summary: v.at("summary", default: ""),
    location: v.at("location", default: ""),
    highlights: v.at("highlights", default: ()),
  )))
}

// Section: awards.
#if "awards" in r and r.awards != none and r.awards.len() > 0 {
  render-award(r.awards.map(a => (
    title: a.at("title", default: ""),
    date: a.at("date", default: ""),
    url: a.at("url", default: ""),
    awarder: a.at("awarder", default: ""),
    summary: a.at("summary", default: ""),
  )))
}

// Section: certificates.
#if "certificates" in r and r.certificates != none and r.certificates.len() > 0 {
  render-certificate(r.certificates.map(c => (
    name: c.at("name", default: ""),
    issuer: c.at("issuer", default: ""),
    url: c.at("url", default: ""),
    date: c.at("date", default: ""),
  )))
}

// Section: publications.
#if "publications" in r and r.publications != none and r.publications.len() > 0 {
  render-publication(r.publications.map(pub => (
    name: pub.at("name", default: ""),
    publisher: pub.at("publisher", default: ""),
    releaseDate: pub.at("releaseDate", default: ""),
    url: pub.at("url", default: ""),
    summary: pub.at("summary", default: ""),
  )))
}

// Section: skills. fantastic-cv has no dedicated skills builder; we
// surface the JSON Resume `skills` array via `render-custom` as a
// "Skills" custom section. Each skill entry becomes one highlight
// where `summary` is the skill name and `description` is the joined
// keywords. If `skills` is empty or missing, emit nothing.
#if "skills" in r and r.skills != none and r.skills.len() > 0 {
  render-custom((
    title: "Skills",
    highlights: r.skills.map(s => (
      summary: s.at("name", default: ""),
      description: s.at("keywords", default: ()).join(", "),
    )),
  ))
}
