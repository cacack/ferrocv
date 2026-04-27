// ferrocv glue entrypoint for the vendored `basic-resume` theme.
//
// This file is authored by ferrocv (NOT vendored). It imports the patched
// upstream `basic-resume.typ` builders and adapts JSON Resume v1.0.0 data
// into basic-resume's helper-function call shape (`resume`, `edu`, `work`,
// `project`, `certificates`). The vendored file carries one §6.1 patch
// (the `@preview/scienceicons` strip); see VENDORING.md.
//
// CONSTITUTION §1: JSON Resume is the canonical input; every field is
// optional per the v1.0.0 schema. Every read uses `.at(..., default: ...)`.
// CONSTITUTION §5: no section-toggle knobs; every section we render in v1
// (basics, work, education, projects, certificates) renders when the data
// is present. Volunteer / awards / publications / skills are deferred to
// a follow-up issue — basic-resume has no native helper for them.

#import "basic-resume.typ": resume, edu, work, project, certificates, dates-helper

// The FerrocvWorld serves the JSON Resume bytes at the virtual path
// "/resume.json". Same convention as every other ferrocv theme.
#let r = json("/resume.json")

// Strip a leading `https://` or `http://` from a URL so it slots into
// basic-resume's helpers, which prepend `https://` themselves (see
// `contact-item(..., link-type: "https://")` and the `project`/
// `certificates` helpers in the upstream source). Returns the input
// untouched if no scheme is present.
#let __strip_scheme(u) = {
  if u.starts-with("https://") { u.slice(8) }
  else if u.starts-with("http://") { u.slice(7) }
  else { u }
}

// Format a JSON Resume startDate/endDate pair using basic-resume's
// `dates-helper` (which renders the en-dash with ligatures disabled).
// Empty range → "" so the helpers' empty-string guards kick in.
#let __fmt_range(start, end) = {
  if start != "" and end != "" { dates-helper(start-date: start, end-date: end) }
  else if start != "" { dates-helper(start-date: start, end-date: "Present") }
  else if end != "" { end }
  else { "" }
}

// Optional-field shim for `basics` — absent, null, or partially-filled
// sub-objects must all render without a dict-lookup panic.
#let basics = r.at("basics", default: (:))
#let location_dict = basics.at("location", default: (:))
#let city = location_dict.at("city", default: "")
#let region = location_dict.at("region", default: "")
#let address = if city != "" and region != "" {
  city + ", " + region
} else if city != "" {
  city
} else {
  region
}

// Profiles → flat github / linkedin args. basic-resume takes them by
// network name (no `profiles` argument), so case-insensitively pluck
// the first matching entry. Anything else (e.g. Mastodon, GitLab) is
// dropped in v1; iterate when a caller asks (CONSTITUTION §5).
//
// `lower()` is a global function on `str` content in Typst, not a
// method — `s.lower()` and `s.to-lowercase()` both error out.
#let __profile_url(network_name) = {
  let profiles = basics.at("profiles", default: ())
  let target = lower(network_name.replace(" ", ""))
  let matches = profiles.filter(p => {
    lower(p.at("network", default: "").replace(" ", "")) == target
  })
  if matches.len() > 0 {
    let p = matches.at(0)
    let u = p.at("url", default: "")
    if u != "" { __strip_scheme(u) }
    else { p.at("username", default: "") }
  } else { "" }
}

// Apply basic-resume's `resume(...)` show rule.
//
// Font override: basic-resume's upstream default ("New Computer Modern")
// is bundled in `typst-assets::fonts()` (same as `typst-jsonresume-cv`),
// so we do NOT override it here — the golden stays reproducible across
// hosts without a font-pin patch.
//
// CONSTITUTION §5: no other overrides. Iterate when a caller asks.
#show: resume.with(
  author: basics.at("name", default: ""),
  location: address,
  email: basics.at("email", default: ""),
  phone: basics.at("phone", default: ""),
  personal-site: __strip_scheme(basics.at("url", default: "")),
  github: __profile_url("GitHub"),
  linkedin: __profile_url("LinkedIn"),
)

// `basics.summary` → an unlabeled paragraph immediately under the
// contact header. basic-resume has no helper for this; emit it as a
// plain paragraph so search-friendly ATS systems still see it.
#{
  let summary = basics.at("summary", default: "")
  if summary != "" [
    #summary
  ]
}

// Section: education → `= Education`. basic-resume's `edu` takes
// `degree` as a single string; compose from JSON Resume `studyType` +
// `area`. `gpa` ← `score`. JSON Resume has no education-level
// `location`; default to empty.
#if "education" in r and r.education != none and r.education.len() > 0 [
  == Education

  #for e in r.education {
    let study_type = e.at("studyType", default: "")
    let area = e.at("area", default: "")
    let degree = if study_type != "" and area != "" { study_type + ", " + area }
      else if study_type != "" { study_type }
      else { area }
    edu(
      institution: e.at("institution", default: ""),
      dates: __fmt_range(e.at("startDate", default: ""), e.at("endDate", default: "")),
      degree: degree,
      gpa: e.at("score", default: ""),
      location: e.at("location", default: ""),
    )
    let courses = e.at("courses", default: ())
    if courses.len() > 0 [
      - Courses: #courses.join(", ")
    ]
  }
]

// Section: work → `= Experience`. basic-resume's `work` takes `title`
// (the role) and `company` (the employer); JSON Resume `position` →
// `title`, `name` → `company`. `summary` and `highlights` go into a
// bullet list under the `work` row, mirroring upstream's example.
#if "work" in r and r.work != none and r.work.len() > 0 [
  == Experience

  #for w in r.work {
    work(
      title: w.at("position", default: ""),
      dates: __fmt_range(w.at("startDate", default: ""), w.at("endDate", default: "")),
      company: w.at("name", default: ""),
      location: w.at("location", default: ""),
    )
    let summary = w.at("summary", default: "")
    let highlights = w.at("highlights", default: ())
    if summary != "" or highlights.len() > 0 [
      #if summary != "" [
        - #summary
      ]
      #for h in highlights [
        - #h
      ]
    ]
  }
]

// Section: projects → `= Projects`. JSON Resume has no `role` per
// project; the helper accepts `role` as optional and degrades to a
// `*name*` heading when it's empty. `description` and `highlights`
// become bullet rows.
#if "projects" in r and r.projects != none and r.projects.len() > 0 [
  == Projects

  #for p in r.projects {
    project(
      name: p.at("name", default: ""),
      url: __strip_scheme(p.at("url", default: "")),
      dates: __fmt_range(p.at("startDate", default: ""), p.at("endDate", default: "")),
    )
    let description = p.at("description", default: "")
    let highlights = p.at("highlights", default: ())
    if description != "" or highlights.len() > 0 [
      #if description != "" [
        - #description
      ]
      #for h in highlights [
        - #h
      ]
    ]
  }
]

// Section: certificates → `= Certificates`. basic-resume's
// `certificates` helper maps 1:1 to JSON Resume's certificate fields.
#if "certificates" in r and r.certificates != none and r.certificates.len() > 0 [
  == Certificates

  #for c in r.certificates {
    certificates(
      name: c.at("name", default: ""),
      issuer: c.at("issuer", default: ""),
      url: __strip_scheme(c.at("url", default: "")),
      date: c.at("date", default: ""),
    )
  }
]
