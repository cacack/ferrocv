// A starter ferrocv native theme — scaffolded by `ferrocv themes new`.
//
// This is YOUR theme. Edit it freely: the layout below is a minimal,
// readable starting point that renders the major JSON Resume sections.
// Add sections, restyle, or rip it apart — it is a single self-contained
// `.typ` file with no external dependencies beyond ferrocv's prelude.
//
// How it works
// ------------
// ferrocv hands every theme two virtual files:
//   - `/resume.json`            — the JSON Resume document being rendered
//   - `/themes/_prelude/lib.typ` — ferrocv's shared native-theme prelude
// The prelude (imported just below) is the native-theme contract from
// CONSTITUTION §4: defensive *data access* only (it ships no layout).
// JSON Resume v1.0.0 has zero required fields, so read everything
// through these helpers — they never error on a missing or oddly-shaped
// key:
//   opt(d, k)            -> d.k if present, else none
//   nz(s)                -> none for none/"" (collapse empties)
//   items(d, k)          -> the array at k, or () when absent/non-array
//   date_range(item)     -> "2019 - 2022" / "2019 - Present" / none
//   join_present(xs, ", ") -> xs joined, dropping none/"" (— "" if all gone)
//   ext(d, "ns")         -> the `x-ns` extension field, or none (§1)
//
// Render it:
//   ferrocv render resume.json --theme ./<this-dir>/resume.typ -o out.pdf
// Swap `--format text` / `--format html` to target the other outputs —
// but note this starter is tuned for PDF. See the authoring guide for
// format-specific concerns.

#import "/themes/_prelude/lib.typ": *

#let resume = json("/resume.json")

// --- Layout helpers (yours to change — layout lives in the theme, §4) -

// A section: a bold heading with a rule beneath it, then the body.
#let section(title, body) = {
  v(7pt)
  text(weight: "bold", size: 11pt, tracking: 0.4pt)[#upper(title)]
  v(1pt)
  line(length: 100%, stroke: 0.5pt)
  v(3pt)
  body
}

// An entry header: title (bold) left, dates (italic) flush right.
// Either side may be absent.
#let entry_head(title, dates) = {
  grid(
    columns: (1fr, auto),
    align: (left, right),
    if title != none { text(weight: "bold")[#title] } else { [] },
    if dates != none { text(style: "italic", size: 9.5pt)[#dates] } else { [] },
  )
}

// --- Page setup -----------------------------------------------------
// "Libertinus Serif" ships in ferrocv's bundled fonts and is Typst's
// default, so output is reproducible on any host with no system-font
// dependency (CONSTITUTION §6). Pick another family only if you vendor
// it; otherwise rendering may vary machine to machine.
#set page(margin: (x: 0.9in, y: 0.8in), numbering: none)
#set text(font: "Libertinus Serif", size: 10.5pt)
#set par(justify: false, leading: 0.62em, spacing: 0.62em)
#set list(indent: 4pt, body-indent: 5pt, spacing: 0.55em)

// --- Header ---------------------------------------------------------
#let basics = opt(resume, "basics")
#if basics != none {
  let name = nz(opt(basics, "name"))
  if name != none {
    align(center, text(size: 20pt, weight: "bold")[#name])
  }
  let label = nz(opt(basics, "label"))
  if label != none {
    align(center, text(size: 11.5pt, style: "italic")[#label])
  }

  // Contact line: email · phone · website. join_present drops any that
  // are absent and returns "" if all are — so guard with `!= ""`.
  let contact = join_present(
    (nz(opt(basics, "email")), nz(opt(basics, "phone")), nz(opt(basics, "url"))),
    "  ·  ",
  )
  if contact != "" {
    align(center, text(size: 9.5pt)[#contact])
  }

  // Extension fields (§1): read author-defined `x-<namespace>` data with
  // `ext`. Here we surface a `meta.x-audience` tag (set by a tailored
  // cut) as a tagline. Delete if you don't use it.
  let audience = ext(opt(resume, "meta"), "audience")
  if audience != none and type(audience) == str and audience != "" {
    v(2pt)
    align(center, text(size: 9.5pt, style: "italic")[Tailored for: #audience])
  }
}

// --- Summary --------------------------------------------------------
#let summary = if basics != none { nz(opt(basics, "summary")) } else { none }
#if summary != none {
  section("Summary", [#summary])
}

// --- Experience -----------------------------------------------------
#let work = items(resume, "work")
#if work.len() > 0 {
  section("Experience", {
    for (i, entry) in work.enumerate() {
      let name = nz(opt(entry, "name"))
      let position = nz(opt(entry, "position"))
      let title = if position != none and name != none {
        position + ", " + name
      } else if position != none {
        position
      } else { name }
      entry_head(title, date_range(entry))
      let wsum = nz(opt(entry, "summary"))
      if wsum != none { text(size: 10pt)[#wsum]; linebreak() }
      // highlights is an array of strings; filter empties before listing.
      let highlights = items(entry, "highlights").filter(h => h != none and h != "")
      if highlights.len() > 0 {
        list(..highlights.map(h => [#h]))
      }
      if i < work.len() - 1 { v(4pt) }
    }
  })
}

// --- Education ------------------------------------------------------
#let education = items(resume, "education")
#if education.len() > 0 {
  section("Education", {
    for (i, entry) in education.enumerate() {
      let institution = nz(opt(entry, "institution"))
      entry_head(institution, date_range(entry))
      let degree = join_present(
        (nz(opt(entry, "studyType")), nz(opt(entry, "area"))),
        ", ",
      )
      if degree != "" { text(size: 10pt)[#degree]; linebreak() }
      if i < education.len() - 1 { v(4pt) }
    }
  })
}

// --- Skills ---------------------------------------------------------
#let skills = items(resume, "skills")
#if skills.len() > 0 {
  section("Skills", {
    for skill in skills {
      let name = nz(opt(skill, "name"))
      let keywords = items(skill, "keywords").filter(k => k != none and k != "")
      let kws = if keywords.len() > 0 { keywords.join(", ") } else { none }
      if name != none {
        text(weight: "semibold")[#name]
        if kws != none { [: #kws] }
        linebreak()
      } else if kws != none {
        [#kws]; linebreak()
      }
    }
  })
}

// --- Projects -------------------------------------------------------
#let projects = items(resume, "projects")
#if projects.len() > 0 {
  section("Projects", {
    for (i, entry) in projects.enumerate() {
      let name = nz(opt(entry, "name"))
      entry_head(name, date_range(entry))
      let desc = nz(opt(entry, "description"))
      if desc != none { text(size: 10pt)[#desc]; linebreak() }
      let highlights = items(entry, "highlights").filter(h => h != none and h != "")
      if highlights.len() > 0 {
        list(..highlights.map(h => [#h]))
      }
      if i < projects.len() - 1 { v(4pt) }
    }
  })
}

// Add more sections the same way — `volunteer`, `awards`, `certificates`,
// `publications`, `languages`, `interests`, `references` are all arrays
// you can iterate with `items(resume, "<name>")`. See the bundled
// `classic` theme for a fuller, every-section example.
