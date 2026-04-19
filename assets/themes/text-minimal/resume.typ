// text-minimal — a native ferrocv theme optimized for plain-text
// extraction via the Frame-walk extractor in `crate::render::compile_text`.
//
// Design constraints (see CLAUDE.md, CONSTITUTION.md §3, §4):
// - Single column. No tables, no grids, no figures, no images, no
//   columns(). Multi-column layout produces zig-zag reading order
//   under y-then-x sort.
// - Plain ASCII only. No bullets like `•`, no arrows, no decorative
//   glyphs — those survive frame extraction and add ATS noise.
// - Generous `linebreak()` and `parbreak()` so visual lines in the
//   compiled frame map cleanly onto extracted text lines.
// - Default font and size — no font-family directives so the output
//   is reproducible across hosts (CONSTITUTION §6).
// - Defensive optional-field reads — JSON Resume v1.0.0 has zero
//   required fields; every accessor must tolerate missing keys.

#let resume = json("/resume.json")

// --- Optional-field helpers ----------------------------------------
//
// `opt(d, k)` returns `d.at(k)` if `d` is a dictionary and `k` is
// present, otherwise `none`. Lets per-section code stay readable
// without sprinkling `if "x" in d { ... }` everywhere.
#let opt(d, k) = if type(d) == dictionary and k in d { d.at(k) } else { none }

// `nz(s)` collapses both absent and empty-string values to `none` so
// sections can uniformly check `if value != none`.
#let nz(s) = if s == none or s == "" { none } else { s }

// Join a list of optional strings with `sep`, dropping `none`/empty.
// Used for location ("city, region, country") where any subset of
// components may be missing.
#let join_present(parts, sep) = {
  let kept = parts.filter(p => p != none and p != "")
  kept.join(sep)
}

// Format a date range. Either bound may be missing; an absent
// `endDate` becomes "Present". Returns `none` if both are absent so
// the caller can skip the line entirely.
#let date_range(item) = {
  let start = nz(opt(item, "startDate"))
  let end = nz(opt(item, "endDate"))
  if start == none and end == none {
    none
  } else if start != none and end != none {
    start + " - " + end
  } else if start != none {
    start + " - Present"
  } else {
    end
  }
}

// --- Page setup ----------------------------------------------------
//
// 1in margins keep things readable in PDF preview without affecting
// extraction. Default font (no `set text(font: ...)` call) — the
// typst-assets default ships with the binary.
#set page(margin: 1in, header: none, footer: none, numbering: none)
#set par(justify: false, leading: 0.65em)

// --- Header --------------------------------------------------------
#let basics = opt(resume, "basics")
#if basics != none {
  let name = nz(opt(basics, "name"))
  let label = nz(opt(basics, "label"))
  let email = nz(opt(basics, "email"))
  let phone = nz(opt(basics, "phone"))
  let url = nz(opt(basics, "url"))
  let location = opt(basics, "location")
  let location_line = if location != none {
    let city = nz(opt(location, "city"))
    let region = nz(opt(location, "region"))
    let country = nz(opt(location, "countryCode"))
    let joined = join_present((city, region, country), ", ")
    if joined == "" { none } else { joined }
  } else { none }

  if name != none {
    text(weight: "bold", size: 14pt)[#name]
    linebreak()
  }
  for line in (label, email, phone, url, location_line) {
    if line != none {
      [#line]
      linebreak()
    }
  }
  // basics.profiles — emit each as a contact-style line. Treated as
  // header continuation rather than a separate section so the
  // rendering matches how social profiles read on a real resume.
  let profiles = opt(basics, "profiles")
  if profiles != none and type(profiles) == array {
    for profile in profiles {
      let network = nz(opt(profile, "network"))
      let username = nz(opt(profile, "username"))
      let url = nz(opt(profile, "url"))
      let label_part = if network != none and username != none {
        network + ": " + username
      } else if network != none {
        network
      } else if username != none {
        username
      } else { none }
      let line = if label_part != none and url != none {
        label_part + " - " + url
      } else if label_part != none {
        label_part
      } else if url != none {
        url
      } else { none }
      if line != none {
        [#line]
        linebreak()
      }
    }
  }
  parbreak()
}

// --- Summary -------------------------------------------------------
#let summary = if basics != none { nz(opt(basics, "summary")) } else { none }
#if summary != none {
  text(weight: "bold")[Summary]
  parbreak()
  [#summary]
  parbreak()
}

// --- Work ----------------------------------------------------------
#let work = opt(resume, "work")
#if work != none and type(work) == array and work.len() > 0 {
  text(weight: "bold")[Work]
  parbreak()
  for entry in work {
    let name = nz(opt(entry, "name"))
    let position = nz(opt(entry, "position"))
    let header = if name != none and position != none {
      position + " - " + name
    } else if name != none {
      name
    } else if position != none {
      position
    } else { none }
    if header != none {
      text(weight: "bold")[#header]
      linebreak()
    }
    let dates = date_range(entry)
    if dates != none {
      [#dates]
      linebreak()
    }
    let work_summary = nz(opt(entry, "summary"))
    if work_summary != none {
      [#work_summary]
      linebreak()
    }
    let highlights = opt(entry, "highlights")
    if highlights != none and type(highlights) == array {
      for h in highlights {
        if h != none and h != "" {
          // Pre-build the prefixed string in code mode so the literal
          // "- " never enters Typst markup mode, where it would be
          // parsed as a list item and rendered with a `•` bullet.
          // Bullets survive frame extraction and add ATS noise.
          let prefixed = "- " + h
          [#prefixed]
          linebreak()
        }
      }
    }
    parbreak()
  }
}

// --- Education -----------------------------------------------------
#let education = opt(resume, "education")
#if education != none and type(education) == array and education.len() > 0 {
  text(weight: "bold")[Education]
  parbreak()
  for entry in education {
    let institution = nz(opt(entry, "institution"))
    if institution != none {
      text(weight: "bold")[#institution]
      linebreak()
    }
    let study_type = nz(opt(entry, "studyType"))
    let area = nz(opt(entry, "area"))
    let study_line = join_present((study_type, area), ", ")
    if study_line != "" {
      [#study_line]
      linebreak()
    }
    let dates = date_range(entry)
    if dates != none {
      [#dates]
      linebreak()
    }
    parbreak()
  }
}

// --- Skills --------------------------------------------------------
#let skills = opt(resume, "skills")
#if skills != none and type(skills) == array and skills.len() > 0 {
  text(weight: "bold")[Skills]
  parbreak()
  for skill in skills {
    let name = nz(opt(skill, "name"))
    let level = nz(opt(skill, "level"))
    let keywords = opt(skill, "keywords")
    let keywords_str = if keywords != none and type(keywords) == array and keywords.len() > 0 {
      keywords.filter(k => k != none and k != "").join(", ")
    } else { none }
    let label_part = if name != none and level != none {
      name + " (" + level + ")"
    } else if name != none {
      name
    } else if level != none {
      level
    } else { none }
    let line = if label_part != none and keywords_str != none and keywords_str != "" {
      label_part + ": " + keywords_str
    } else if label_part != none {
      label_part
    } else if keywords_str != none {
      keywords_str
    } else { none }
    if line != none {
      [#line]
      linebreak()
    }
  }
  parbreak()
}

// --- Projects ------------------------------------------------------
#let projects = opt(resume, "projects")
#if projects != none and type(projects) == array and projects.len() > 0 {
  text(weight: "bold")[Projects]
  parbreak()
  for entry in projects {
    let name = nz(opt(entry, "name"))
    if name != none {
      text(weight: "bold")[#name]
      linebreak()
    }
    let description = nz(opt(entry, "description"))
    if description != none {
      [#description]
      linebreak()
    }
    let url = nz(opt(entry, "url"))
    if url != none {
      [#url]
      linebreak()
    }
    let dates = date_range(entry)
    if dates != none {
      [#dates]
      linebreak()
    }
    parbreak()
  }
}

// --- Volunteer -----------------------------------------------------
#let volunteer = opt(resume, "volunteer")
#if volunteer != none and type(volunteer) == array and volunteer.len() > 0 {
  text(weight: "bold")[Volunteer]
  parbreak()
  for entry in volunteer {
    let organization = nz(opt(entry, "organization"))
    let position = nz(opt(entry, "position"))
    let header = if organization != none and position != none {
      position + " - " + organization
    } else if organization != none {
      organization
    } else if position != none {
      position
    } else { none }
    if header != none {
      text(weight: "bold")[#header]
      linebreak()
    }
    let dates = date_range(entry)
    if dates != none {
      [#dates]
      linebreak()
    }
    let v_summary = nz(opt(entry, "summary"))
    if v_summary != none {
      [#v_summary]
      linebreak()
    }
    let highlights = opt(entry, "highlights")
    if highlights != none and type(highlights) == array {
      for h in highlights {
        if h != none and h != "" {
          let prefixed = "- " + h
          [#prefixed]
          linebreak()
        }
      }
    }
    parbreak()
  }
}

// --- Awards --------------------------------------------------------
#let awards = opt(resume, "awards")
#if awards != none and type(awards) == array and awards.len() > 0 {
  text(weight: "bold")[Awards]
  parbreak()
  for entry in awards {
    let title = nz(opt(entry, "title"))
    if title != none {
      text(weight: "bold")[#title]
      linebreak()
    }
    let awarder = nz(opt(entry, "awarder"))
    let date = nz(opt(entry, "date"))
    let meta = join_present((awarder, date), " - ")
    if meta != "" {
      [#meta]
      linebreak()
    }
    let a_summary = nz(opt(entry, "summary"))
    if a_summary != none {
      [#a_summary]
      linebreak()
    }
    parbreak()
  }
}

// --- Certificates --------------------------------------------------
#let certificates = opt(resume, "certificates")
#if certificates != none and type(certificates) == array and certificates.len() > 0 {
  text(weight: "bold")[Certificates]
  parbreak()
  for entry in certificates {
    let name = nz(opt(entry, "name"))
    if name != none {
      text(weight: "bold")[#name]
      linebreak()
    }
    let issuer = nz(opt(entry, "issuer"))
    let date = nz(opt(entry, "date"))
    let meta = join_present((issuer, date), " - ")
    if meta != "" {
      [#meta]
      linebreak()
    }
    let url = nz(opt(entry, "url"))
    if url != none {
      [#url]
      linebreak()
    }
    parbreak()
  }
}

// --- Publications --------------------------------------------------
#let publications = opt(resume, "publications")
#if publications != none and type(publications) == array and publications.len() > 0 {
  text(weight: "bold")[Publications]
  parbreak()
  for entry in publications {
    let name = nz(opt(entry, "name"))
    if name != none {
      text(weight: "bold")[#name]
      linebreak()
    }
    let publisher = nz(opt(entry, "publisher"))
    let release = nz(opt(entry, "releaseDate"))
    let meta = join_present((publisher, release), " - ")
    if meta != "" {
      [#meta]
      linebreak()
    }
    let url = nz(opt(entry, "url"))
    if url != none {
      [#url]
      linebreak()
    }
    let p_summary = nz(opt(entry, "summary"))
    if p_summary != none {
      [#p_summary]
      linebreak()
    }
    parbreak()
  }
}

// --- Languages -----------------------------------------------------
#let languages = opt(resume, "languages")
#if languages != none and type(languages) == array and languages.len() > 0 {
  text(weight: "bold")[Languages]
  parbreak()
  for entry in languages {
    let language = nz(opt(entry, "language"))
    let fluency = nz(opt(entry, "fluency"))
    let line = if language != none and fluency != none {
      language + " (" + fluency + ")"
    } else if language != none {
      language
    } else if fluency != none {
      fluency
    } else { none }
    if line != none {
      [#line]
      linebreak()
    }
  }
  parbreak()
}

// --- Interests -----------------------------------------------------
#let interests = opt(resume, "interests")
#if interests != none and type(interests) == array and interests.len() > 0 {
  text(weight: "bold")[Interests]
  parbreak()
  for entry in interests {
    let name = nz(opt(entry, "name"))
    let keywords = opt(entry, "keywords")
    let keywords_str = if keywords != none and type(keywords) == array and keywords.len() > 0 {
      keywords.filter(k => k != none and k != "").join(", ")
    } else { none }
    let line = if name != none and keywords_str != none and keywords_str != "" {
      name + ": " + keywords_str
    } else if name != none {
      name
    } else if keywords_str != none {
      keywords_str
    } else { none }
    if line != none {
      [#line]
      linebreak()
    }
  }
  parbreak()
}

// --- References ----------------------------------------------------
#let references = opt(resume, "references")
#if references != none and type(references) == array and references.len() > 0 {
  text(weight: "bold")[References]
  parbreak()
  for entry in references {
    let name = nz(opt(entry, "name"))
    if name != none {
      text(weight: "bold")[#name]
      linebreak()
    }
    let reference = nz(opt(entry, "reference"))
    if reference != none {
      [#reference]
      linebreak()
    }
    parbreak()
  }
}
