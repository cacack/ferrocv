// ferrocv glue entrypoint for the vendored `modern-cv` theme.
//
// This file is authored by ferrocv (NOT vendored). It imports the patched
// upstream `lib.typ` and adapts JSON Resume v1.0.0 data into modern-cv's
// helper-function call shape (`resume`, `resume-entry`, `resume-item`,
// `resume-skill-item`, `resume-certification`). The vendored `lib.typ`
// carries three §6.1 / reproducibility patches; see VENDORING.md.
//
// CONSTITUTION §1: JSON Resume is the canonical input; every field is
// optional per the v1.0.0 schema. Every read uses `.at(..., default: ...)`.
// CONSTITUTION §5: no section-toggle knobs; every section with data renders.

#import "lib.typ": resume, resume-entry, resume-item, resume-skill-item, resume-certification

// The FerrocvWorld serves the JSON Resume bytes at the virtual path
// "/resume.json". Same convention as every other ferrocv theme.
#let r = json("/resume.json")

// Small helper: format a JSON Resume startDate/endDate pair. modern-cv's
// `resume-entry` takes a single `date:` string, so we compose one.
#let __fmt_range(start, end) = {
  if start != "" and end != "" { start + " – " + end }
  else if start != "" { start + " – Present" }
  else if end != "" { end }
  else { "" }
}

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

// ferrocv glue: JSON Resume `basics.name` is a single string; modern-cv's
// `author` dict expects `firstname`/`lastname`. Split on the first run of
// whitespace: first token → firstname, remainder → lastname.
#let full_name = basics.at("name", default: "")
#let name_parts = if full_name != "" { full_name.split(regex("\\s+")) } else { () }
#let firstname = if name_parts.len() > 0 { name_parts.at(0) } else { "" }
#let lastname = if name_parts.len() > 1 { name_parts.slice(1).join(" ") } else { "" }

// ferrocv glue: JSON Resume `basics.label` is a single string; modern-cv's
// `author.positions` expects an array. Wrap in a singleton list when set.
#let label = basics.at("label", default: "")

#let author_dict = (
  firstname: firstname,
  lastname: lastname,
  positions: if label != "" { (label,) } else { () },
  address: address,
  phone: basics.at("phone", default: ""),
  email: basics.at("email", default: ""),
  personal-site: basics.at("url", default: ""),
)

// Apply modern-cv's `resume(...)` show rule.
//
// Font override: modern-cv's upstream defaults (Source Sans Pro / Roboto)
// are NOT bundled in `typst-assets::fonts()`. Libertinus Serif IS bundled;
// overriding makes the golden reproducible across hosts. VENDORING.md has
// the full rationale.
//
// date: "" keeps the footer's date slot blank (see VENDORING.md, Patch C).
//
// profile-picture: none — upstream's default is the `image` *function*
// itself (not an image value), which upstream then branches on via
// `if profile-picture != none`. Passing `none` explicitly avoids the
// "profile-picture function passed as block body" failure that Typst
// 0.13 rejects. (Upstream's own example passes an image path; the
// function-as-default only works if the caller always overrides it.)
//
// CONSTITUTION §5: no other overrides. Iterate when a caller asks.
#show: resume.with(
  author: author_dict,
  profile-picture: none,
  date: "",
  font: "Libertinus Serif",
  header-font: "Libertinus Serif",
)

// Section: work → `= Experience`. JSON Resume `work[i].summary` → the
// `description` slot (the secondary right-header); `highlights` become
// a bullet list wrapped in `resume-item`.
#if "work" in r and r.work != none and r.work.len() > 0 {
  [= Experience]
  for w in r.work {
    resume-entry(
      title: w.at("name", default: ""),
      location: w.at("location", default: ""),
      date: __fmt_range(w.at("startDate", default: ""), w.at("endDate", default: "")),
      description: w.at("position", default: ""),
      title-link: w.at("url", default: none),
    )
    let summary = w.at("summary", default: "")
    let highlights = w.at("highlights", default: ())
    if summary != "" or highlights.len() > 0 {
      resume-item[
        #if summary != "" [#summary]

        #for h in highlights [
          - #h
        ]
      ]
    }
  }
}

// Section: education → `= Education`. JSON Resume has no `location` on
// education entries; default to empty. Description composes studyType
// and area, trimming stray separator if either is missing.
#if "education" in r and r.education != none and r.education.len() > 0 {
  [= Education]
  for e in r.education {
    let study_type = e.at("studyType", default: "")
    let area = e.at("area", default: "")
    let desc = if study_type != "" and area != "" { study_type + ", " + area }
      else if study_type != "" { study_type }
      else { area }
    resume-entry(
      title: e.at("institution", default: ""),
      location: e.at("location", default: ""),
      date: __fmt_range(e.at("startDate", default: ""), e.at("endDate", default: "")),
      description: desc,
      title-link: e.at("url", default: none),
    )
    let courses = e.at("courses", default: ())
    if courses.len() > 0 {
      resume-item[
        Courses: #courses.join(", ")
      ]
    }
  }
}

// Section: projects → `= Projects`. JSON Resume has no single-string
// "location" on projects; use roles (or keywords as a fallback) in that
// slot — mirrors the fantastic-cv glue's choice.
#if "projects" in r and r.projects != none and r.projects.len() > 0 {
  [= Projects]
  for p in r.projects {
    let roles = p.at("roles", default: p.at("keywords", default: ()))
    resume-entry(
      title: p.at("name", default: ""),
      location: roles.join(", "),
      date: __fmt_range(p.at("startDate", default: ""), p.at("endDate", default: "")),
      description: p.at("description", default: ""),
      title-link: p.at("url", default: none),
    )
    let highlights = p.at("highlights", default: ())
    if highlights.len() > 0 {
      resume-item[
        #for h in highlights [
          - #h
        ]
      ]
    }
  }
}

// Section: volunteer → `= Volunteer`.
#if "volunteer" in r and r.volunteer != none and r.volunteer.len() > 0 {
  [= Volunteer]
  for v in r.volunteer {
    resume-entry(
      title: v.at("organization", default: ""),
      location: "",
      date: __fmt_range(v.at("startDate", default: ""), v.at("endDate", default: "")),
      description: v.at("position", default: ""),
      title-link: v.at("url", default: none),
    )
    let summary = v.at("summary", default: "")
    let highlights = v.at("highlights", default: ())
    if summary != "" or highlights.len() > 0 {
      resume-item[
        #if summary != "" [#summary]

        #for h in highlights [
          - #h
        ]
      ]
    }
  }
}

// Section: awards → `= Awards`. JSON Resume `awards[i].awarder` occupies
// modern-cv's `location` slot; `.date` is already a plain string.
#if "awards" in r and r.awards != none and r.awards.len() > 0 {
  [= Awards]
  for a in r.awards {
    resume-entry(
      title: a.at("title", default: ""),
      location: a.at("awarder", default: ""),
      date: a.at("date", default: ""),
      description: a.at("summary", default: ""),
      title-link: a.at("url", default: none),
    )
  }
}

// Section: certificates → `= Certifications`.
// ferrocv glue: modern-cv's `resume-certification` is a two-arg function
// (`certification`, `date`). JSON Resume's `issuer` and `url` are dropped
// here; iterate when a caller asks (CONSTITUTION §5).
#if "certificates" in r and r.certificates != none and r.certificates.len() > 0 {
  [= Certifications]
  for c in r.certificates {
    resume-certification(
      c.at("name", default: ""),
      c.at("date", default: ""),
    )
  }
}

// Section: publications → `= Publications`. JSON Resume `releaseDate`
// maps to modern-cv's `date:`, `publisher` to `location:`.
#if "publications" in r and r.publications != none and r.publications.len() > 0 {
  [= Publications]
  for pub in r.publications {
    resume-entry(
      title: pub.at("name", default: ""),
      location: pub.at("publisher", default: ""),
      date: pub.at("releaseDate", default: ""),
      description: pub.at("summary", default: ""),
      title-link: pub.at("url", default: none),
    )
  }
}

// Section: skills → `= Skills`. Each JSON Resume skill entry becomes one
// `resume-skill-item(<name>, <keywords>)` row. modern-cv's signature is
// two positional args (`category`, `items`).
#if "skills" in r and r.skills != none and r.skills.len() > 0 {
  [= Skills]
  for s in r.skills {
    resume-skill-item(
      s.at("name", default: ""),
      s.at("keywords", default: ()),
    )
  }
}
