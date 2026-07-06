// classic — a PDF-first native ferrocv theme (CONSTITUTION §4).
//
// Authored directly against the JSON Resume v1.0.0 schema via the shared
// prelude (`/themes/_prelude/lib.typ`), and designed to look good on the
// page as a PDF — unlike `text-minimal` (tuned for frame-extracted plain
// text) and `html-minimal` (semantic HTML). This is the proof that the
// prelude contract supports real layout, not just minimal output.
//
// Design constraints (see CLAUDE.md, CONSTITUTION.md §3, §4, §6):
// - Single column, classic serif look. Uses "Libertinus Serif", which
//   ships in `typst-assets` (the only bundled non-mono family) and is
//   Typst's default — so output is reproducible across hosts with no
//   system-font dependency (§6). No `@preview/...` imports (§6.1).
// - Defensive optional-field reads — every accessor comes from the
//   prelude (`opt`, `nz`, `items`, `date_range`, `join_present`) and
//   tolerates missing keys; JSON Resume has zero required fields, so the
//   sparse fixture (basics + work only) must render cleanly.
// - Layout lives here, not in the prelude: section headings, the rule,
//   and the title-left / dates-right entry header are this theme's own
//   (the prelude is data access only — §4).
//
// Audience-aware rendering (demonstrates the `ext` accessor, #180): if
// the document's `meta` object carries an `x-audience` string — e.g. a
// tailored cut stamped "security" — it is surfaced as a "Tailored for:
// <label>" tagline under the header. The tag lives under `meta` (not the
// document root) because JSON Resume's schema forbids unknown properties
// at the root but permits `x-` extensions inside objects like `meta`.
// Absent `meta.x-audience` ⇒ no tagline.
//
// The MIT-licensed source under `assets/themes/classic/` is also
// redistributable under the crate's MIT-or-Apache-2.0 dual license; the
// sibling `LICENSE` is duplicated so the theme stays self-contained if it
// is ever extracted into its own package.

#import "/themes/_prelude/lib.typ": *

#let resume = json("/resume.json")

// --- Layout helpers (theme-scoped; layout stays per-theme, §4) ------

// A section: a spaced, letter-tracked bold heading with a full-width
// rule beneath it, then the section body. The leading space before the
// heading sets the gap between sections.
#let section(title, body) = {
  v(14pt)
  text(weight: "bold", size: 11pt, tracking: 0.4pt)[#upper(title)]
  v(1pt)
  line(length: 100%, stroke: 0.5pt)
  v(3pt)
  body
}

// Keep a single entry (its header and body) together: wrap it in a
// non-breakable, full-width block so the title never strands at the
// bottom of one page with its highlights pushed onto the next. The
// explicit `width: 100%` is required — a default-width block shrinks to
// fit its content, which would collapse the `1fr` dates-flush-right
// grid in `entry_head`.
#let entry_block(body) = block(breakable: false, width: 100%, body)

// An entry header: title (bold) on the left, dates (italic) flush right
// on the same row. Either side may be absent.
#let entry_head(title, dates) = {
  grid(
    columns: (1fr, auto),
    align: (left, right),
    if title != none { text(weight: "bold")[#title] } else { [] },
    if dates != none { text(style: "italic", size: 9.5pt)[#dates] } else { [] },
  )
}

// --- Page setup -----------------------------------------------------
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

  let email = nz(opt(basics, "email"))
  let phone = nz(opt(basics, "phone"))
  let url = nz(opt(basics, "url"))
  let location = opt(basics, "location")
  let loc_line = if location != none {
    let city = nz(opt(location, "city"))
    let region = nz(opt(location, "region"))
    let country = nz(opt(location, "countryCode"))
    let joined = join_present((city, region, country), ", ")
    if joined == "" { none } else { joined }
  } else { none }
  let contact = join_present((email, phone, url, loc_line), "  ·  ")
  if contact != "" {
    align(center, text(size: 9.5pt)[#contact])
  }

  // Profiles render as one centered, separated line.
  let profile_lines = ()
  for profile in items(basics, "profiles") {
    let network = nz(opt(profile, "network"))
    let username = nz(opt(profile, "username"))
    let purl = nz(opt(profile, "url"))
    let label_part = if network != none and username != none {
      network + ": " + username
    } else if network != none {
      network
    } else if username != none {
      username
    } else { none }
    let entry = if label_part != none and purl != none {
      label_part + " (" + purl + ")"
    } else if label_part != none {
      label_part
    } else if purl != none {
      purl
    } else { none }
    if entry != none { profile_lines.push(entry) }
  }
  if profile_lines.len() > 0 {
    align(center, text(size: 9.5pt)[#profile_lines.join("  ·  ")])
  }

  // Audience tagline — read the `x-audience` extension from the `meta`
  // sub-dict (the schema's `meta` object permits `x-` extensions; the
  // document root forbids extras). Pass the bare namespace `"audience"`
  // to `ext`, which prepends the `x-` prefix — never a dotted path.
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
      entry_block({
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
        let highlights = items(entry, "highlights").filter(h => h != none and h != "")
        if highlights.len() > 0 {
          list(..highlights.map(h => [#h]))
        }
      })
      if i < work.len() - 1 { v(4pt) }
    }
  })
}

// --- Education ------------------------------------------------------
#let education = items(resume, "education")
#if education.len() > 0 {
  section("Education", {
    for (i, entry) in education.enumerate() {
      entry_block({
        // Renders institution, study type/area, and dates. `score`,
        // `courses`, and `url` are deliberately omitted to keep the entry
        // compact; add them here if a fuller education block is wanted.
        let institution = nz(opt(entry, "institution"))
        entry_head(institution, date_range(entry))
        let study_type = nz(opt(entry, "studyType"))
        let area = nz(opt(entry, "area"))
        let degree = join_present((study_type, area), ", ")
        if degree != "" { text(size: 10pt)[#degree]; linebreak() }
      })
      if i < education.len() - 1 { v(4pt) }
    }
  })
}

// --- Projects -------------------------------------------------------
#let projects = items(resume, "projects")
#if projects.len() > 0 {
  section("Projects", {
    for (i, entry) in projects.enumerate() {
      entry_block({
        let name = nz(opt(entry, "name"))
        entry_head(name, date_range(entry))
        let desc = nz(opt(entry, "description"))
        if desc != none { text(size: 10pt)[#desc]; linebreak() }
        let url = nz(opt(entry, "url"))
        if url != none { text(size: 9pt, style: "italic")[#url]; linebreak() }
        let highlights = items(entry, "highlights").filter(h => h != none and h != "")
        if highlights.len() > 0 {
          list(..highlights.map(h => [#h]))
        }
      })
      if i < projects.len() - 1 { v(4pt) }
    }
  })
}

// --- Skills ---------------------------------------------------------
#let skills = items(resume, "skills")
#if skills.len() > 0 {
  section("Skills", {
    for skill in skills {
      let name = nz(opt(skill, "name"))
      let level = nz(opt(skill, "level"))
      let keywords = items(skill, "keywords").filter(k => k != none and k != "")
      let kws = if keywords.len() > 0 { keywords.join(", ") } else { none }
      let label_part = if name != none and level != none {
        name + " (" + level + ")"
      } else if name != none {
        name
      } else { level }
      if label_part != none {
        text(weight: "semibold")[#label_part]
        if kws != none { [: #kws] }
        linebreak()
      } else if kws != none {
        [#kws]
        linebreak()
      }
    }
  })
}

// --- Volunteer ------------------------------------------------------
#let volunteer = items(resume, "volunteer")
#if volunteer.len() > 0 {
  section("Volunteer", {
    for (i, entry) in volunteer.enumerate() {
      entry_block({
        let organization = nz(opt(entry, "organization"))
        let position = nz(opt(entry, "position"))
        let title = if position != none and organization != none {
          position + ", " + organization
        } else if position != none {
          position
        } else { organization }
        entry_head(title, date_range(entry))
        let vsum = nz(opt(entry, "summary"))
        if vsum != none { text(size: 10pt)[#vsum]; linebreak() }
        let highlights = items(entry, "highlights").filter(h => h != none and h != "")
        if highlights.len() > 0 {
          list(..highlights.map(h => [#h]))
        }
      })
      if i < volunteer.len() - 1 { v(4pt) }
    }
  })
}

// --- Awards ---------------------------------------------------------
#let awards = items(resume, "awards")
#if awards.len() > 0 {
  section("Awards", {
    for (i, entry) in awards.enumerate() {
      entry_block({
        let title = nz(opt(entry, "title"))
        let date = nz(opt(entry, "date"))
        entry_head(title, date)
        let awarder = nz(opt(entry, "awarder"))
        if awarder != none { text(size: 10pt)[#awarder]; linebreak() }
        let asum = nz(opt(entry, "summary"))
        if asum != none { text(size: 10pt)[#asum] }
      })
      if i < awards.len() - 1 { v(4pt) }
    }
  })
}

// --- Certificates ---------------------------------------------------
#let certificates = items(resume, "certificates")
#if certificates.len() > 0 {
  section("Certificates", {
    for (i, entry) in certificates.enumerate() {
      entry_block({
        let name = nz(opt(entry, "name"))
        let date = nz(opt(entry, "date"))
        entry_head(name, date)
        let issuer = nz(opt(entry, "issuer"))
        if issuer != none { text(size: 10pt)[#issuer]; linebreak() }
        let url = nz(opt(entry, "url"))
        if url != none { text(size: 9pt, style: "italic")[#url]; linebreak() }
      })
      if i < certificates.len() - 1 { v(4pt) }
    }
  })
}

// --- Publications ---------------------------------------------------
#let publications = items(resume, "publications")
#if publications.len() > 0 {
  section("Publications", {
    for (i, entry) in publications.enumerate() {
      entry_block({
        let name = nz(opt(entry, "name"))
        let release = nz(opt(entry, "releaseDate"))
        entry_head(name, release)
        let publisher = nz(opt(entry, "publisher"))
        if publisher != none { text(size: 10pt)[#publisher]; linebreak() }
        let psum = nz(opt(entry, "summary"))
        if psum != none { text(size: 10pt)[#psum]; linebreak() }
        let url = nz(opt(entry, "url"))
        if url != none { text(size: 9pt, style: "italic")[#url]; linebreak() }
      })
      if i < publications.len() - 1 { v(4pt) }
    }
  })
}

// --- Languages ------------------------------------------------------
#let languages = items(resume, "languages")
#if languages.len() > 0 {
  // Languages render as one ` · `-separated line ("English (Native) ·
  // German (Reading)") — a conventional resume style, and unambiguous in
  // text extraction (one-entry-per-line risks adjacent short lines being
  // fused by the golden's pdf-extract reader).
  let lang_lines = ()
  for entry in languages {
    let language = nz(opt(entry, "language"))
    let fluency = nz(opt(entry, "fluency"))
    let line = if language != none and fluency != none {
      language + " (" + fluency + ")"
    } else if language != none {
      language
    } else { fluency }
    if line != none { lang_lines.push(line) }
  }
  if lang_lines.len() > 0 {
    section("Languages", [#lang_lines.join("  ·  ")])
  }
}

// --- Interests ------------------------------------------------------
#let interests = items(resume, "interests")
#if interests.len() > 0 {
  section("Interests", {
    for entry in interests {
      let name = nz(opt(entry, "name"))
      let keywords = items(entry, "keywords").filter(k => k != none and k != "")
      let kws = if keywords.len() > 0 { keywords.join(", ") } else { none }
      let line = if name != none and kws != none {
        name + ": " + kws
      } else if name != none {
        name
      } else { kws }
      if line != none { [#line]; linebreak() }
    }
  })
}

// --- References -----------------------------------------------------
#let references = items(resume, "references")
#if references.len() > 0 {
  section("References", {
    for (i, entry) in references.enumerate() {
      entry_block({
        let name = nz(opt(entry, "name"))
        if name != none { text(weight: "bold")[#name]; linebreak() }
        let reference = nz(opt(entry, "reference"))
        if reference != none { text(size: 10pt, style: "italic")[#reference] }
      })
      if i < references.len() - 1 { v(4pt) }
    }
  })
}
