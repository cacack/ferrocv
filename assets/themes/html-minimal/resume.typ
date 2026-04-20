// html-minimal — a native ferrocv theme authored directly against
// JSON Resume v1.0.0 for the `--format html` target.
//
// Design constraints (see CLAUDE.md, CONSTITUTION.md §3, §4, §6.1):
// - HTML-only. No `set page(...)` / `set par(...)` — Typst 0.14 warns
//   when paged-layout rules are used in HTML mode, and this theme
//   never renders to PDF. Text output still goes through the
//   sibling `text-minimal` theme which remains optimized for the
//   frame-walk extractor.
// - Semantic-HTML first. `<h2>` section headings, `<section>` per
//   JSON Resume area, `<header>` for the contact block, `<a href>`
//   for every URL (including `mailto:` and `tel:`), `<ul>`/`<li>`
//   for highlight lists. The browser provides visual affordances
//   via HTML semantics — no inline CSS, no decorative glyphs.
// - Single-file output. No package imports (the embedded
//   `FerrocvWorld` rejects them anyway), no images, no external
//   stylesheets, no web fonts, no inline style attributes, no
//   sourced sub-resources. Resume data never leaves the process.
// - Defensive optional-field reads — JSON Resume v1.0.0 has zero
//   required fields; every accessor must tolerate missing keys.
//   The helper block is duplicated from `text-minimal` rather than
//   shared; CONSTITUTION §5 says share on the third caller.
//
// Typed-HTML API reference: https://typst.app/docs/reference/html/

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

// --- Document ------------------------------------------------------
//
// Single `<main>` wrapping the whole resume. Each JSON Resume area
// becomes its own `<section>` with an `<h2>` heading. Contact block
// is a `<header>` sibling of the sections.
#html.main[
  // --- Header ------------------------------------------------------
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

    html.header[
      #if name != none {
        html.h1[#name]
      }
      #if label != none {
        html.p[#label]
      }
      #if email != none {
        html.p[#html.a(href: "mailto:" + email)[#email]]
      }
      #if phone != none {
        let tel_href = "tel:" + phone.replace(" ", "").replace("-", "")
        html.p[#html.a(href: tel_href)[#phone]]
      }
      #if url != none {
        html.p[#html.a(href: url)[#url]]
      }
      #if location_line != none {
        html.p[#location_line]
      }
      // basics.profiles — emit each as a contact-style paragraph.
      // Treated as header continuation rather than a separate section
      // so the rendering matches how social profiles read on a real
      // resume.
      #let profiles = opt(basics, "profiles")
      #if profiles != none and type(profiles) == array and profiles.len() > 0 {
        html.ul[
          #for profile in profiles {
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
            if label_part != none and purl != none {
              html.li[#label_part - #html.a(href: purl)[#purl]]
            } else if label_part != none {
              html.li[#label_part]
            } else if purl != none {
              html.li[#html.a(href: purl)[#purl]]
            }
          }
        ]
      }
    ]
  }

  // --- Summary -----------------------------------------------------
  #let summary = if basics != none { nz(opt(basics, "summary")) } else { none }
  #if summary != none {
    html.section[
      #html.h2[Summary]
      #html.p[#summary]
    ]
  }

  // --- Work --------------------------------------------------------
  #let work = opt(resume, "work")
  #if work != none and type(work) == array and work.len() > 0 {
    html.section[
      #html.h2[Work]
      #for entry in work {
        let name = nz(opt(entry, "name"))
        let position = nz(opt(entry, "position"))
        let wurl = nz(opt(entry, "url"))
        let header_text = if name != none and position != none {
          position + " - " + name
        } else if name != none {
          name
        } else if position != none {
          position
        } else { none }
        html.article[
          #if header_text != none {
            html.h3[#header_text]
          }
          #if wurl != none {
            html.p[#html.a(href: wurl)[#wurl]]
          }
          #let dates = date_range(entry)
          #if dates != none {
            html.p[#dates]
          }
          #let work_summary = nz(opt(entry, "summary"))
          #if work_summary != none {
            html.p[#work_summary]
          }
          #let highlights = opt(entry, "highlights")
          #if highlights != none and type(highlights) == array {
            let kept = highlights.filter(h => h != none and h != "")
            if kept.len() > 0 {
              html.ul[
                #for h in kept {
                  html.li[#h]
                }
              ]
            }
          }
        ]
      }
    ]
  }

  // --- Volunteer ---------------------------------------------------
  #let volunteer = opt(resume, "volunteer")
  #if volunteer != none and type(volunteer) == array and volunteer.len() > 0 {
    html.section[
      #html.h2[Volunteer]
      #for entry in volunteer {
        let organization = nz(opt(entry, "organization"))
        let position = nz(opt(entry, "position"))
        let vurl = nz(opt(entry, "url"))
        let header_text = if organization != none and position != none {
          position + " - " + organization
        } else if organization != none {
          organization
        } else if position != none {
          position
        } else { none }
        html.article[
          #if header_text != none {
            html.h3[#header_text]
          }
          #if vurl != none {
            html.p[#html.a(href: vurl)[#vurl]]
          }
          #let dates = date_range(entry)
          #if dates != none {
            html.p[#dates]
          }
          #let v_summary = nz(opt(entry, "summary"))
          #if v_summary != none {
            html.p[#v_summary]
          }
          #let highlights = opt(entry, "highlights")
          #if highlights != none and type(highlights) == array {
            let kept = highlights.filter(h => h != none and h != "")
            if kept.len() > 0 {
              html.ul[
                #for h in kept {
                  html.li[#h]
                }
              ]
            }
          }
        ]
      }
    ]
  }

  // --- Education ---------------------------------------------------
  #let education = opt(resume, "education")
  #if education != none and type(education) == array and education.len() > 0 {
    html.section[
      #html.h2[Education]
      #for entry in education {
        let institution = nz(opt(entry, "institution"))
        let eurl = nz(opt(entry, "url"))
        html.article[
          #if institution != none {
            html.h3[#institution]
          }
          #if eurl != none {
            html.p[#html.a(href: eurl)[#eurl]]
          }
          #let study_type = nz(opt(entry, "studyType"))
          #let area = nz(opt(entry, "area"))
          #let study_line = join_present((study_type, area), ", ")
          #if study_line != "" {
            html.p[#study_line]
          }
          #let dates = date_range(entry)
          #if dates != none {
            html.p[#dates]
          }
        ]
      }
    ]
  }

  // --- Awards ------------------------------------------------------
  #let awards = opt(resume, "awards")
  #if awards != none and type(awards) == array and awards.len() > 0 {
    html.section[
      #html.h2[Awards]
      #for entry in awards {
        let title = nz(opt(entry, "title"))
        html.article[
          #if title != none {
            html.h3[#title]
          }
          #let awarder = nz(opt(entry, "awarder"))
          #let date = nz(opt(entry, "date"))
          #let meta = join_present((awarder, date), " - ")
          #if meta != "" {
            html.p[#meta]
          }
          #let a_summary = nz(opt(entry, "summary"))
          #if a_summary != none {
            html.p[#a_summary]
          }
          #let highlights = opt(entry, "highlights")
          #if highlights != none and type(highlights) == array {
            let kept = highlights.filter(h => h != none and h != "")
            if kept.len() > 0 {
              html.ul[
                #for h in kept {
                  html.li[#h]
                }
              ]
            }
          }
        ]
      }
    ]
  }

  // --- Certificates ------------------------------------------------
  #let certificates = opt(resume, "certificates")
  #if certificates != none and type(certificates) == array and certificates.len() > 0 {
    html.section[
      #html.h2[Certificates]
      #for entry in certificates {
        let name = nz(opt(entry, "name"))
        html.article[
          #if name != none {
            html.h3[#name]
          }
          #let issuer = nz(opt(entry, "issuer"))
          #let date = nz(opt(entry, "date"))
          #let meta = join_present((issuer, date), " - ")
          #if meta != "" {
            html.p[#meta]
          }
          #let curl = nz(opt(entry, "url"))
          #if curl != none {
            html.p[#html.a(href: curl)[#curl]]
          }
        ]
      }
    ]
  }

  // --- Publications ------------------------------------------------
  #let publications = opt(resume, "publications")
  #if publications != none and type(publications) == array and publications.len() > 0 {
    html.section[
      #html.h2[Publications]
      #for entry in publications {
        let name = nz(opt(entry, "name"))
        html.article[
          #if name != none {
            html.h3[#name]
          }
          #let publisher = nz(opt(entry, "publisher"))
          #let release = nz(opt(entry, "releaseDate"))
          #let meta = join_present((publisher, release), " - ")
          #if meta != "" {
            html.p[#meta]
          }
          #let purl = nz(opt(entry, "url"))
          #if purl != none {
            html.p[#html.a(href: purl)[#purl]]
          }
          #let p_summary = nz(opt(entry, "summary"))
          #if p_summary != none {
            html.p[#p_summary]
          }
        ]
      }
    ]
  }

  // --- Skills ------------------------------------------------------
  #let skills = opt(resume, "skills")
  #if skills != none and type(skills) == array and skills.len() > 0 {
    html.section[
      #html.h2[Skills]
      #html.ul[
        #for skill in skills {
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
            html.li[#line]
          }
        }
      ]
    ]
  }

  // --- Languages ---------------------------------------------------
  #let languages = opt(resume, "languages")
  #if languages != none and type(languages) == array and languages.len() > 0 {
    html.section[
      #html.h2[Languages]
      #html.ul[
        #for entry in languages {
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
            html.li[#line]
          }
        }
      ]
    ]
  }

  // --- Interests ---------------------------------------------------
  #let interests = opt(resume, "interests")
  #if interests != none and type(interests) == array and interests.len() > 0 {
    html.section[
      #html.h2[Interests]
      #html.ul[
        #for entry in interests {
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
            html.li[#line]
          }
        }
      ]
    ]
  }

  // --- Projects ----------------------------------------------------
  #let projects = opt(resume, "projects")
  #if projects != none and type(projects) == array and projects.len() > 0 {
    html.section[
      #html.h2[Projects]
      #for entry in projects {
        let name = nz(opt(entry, "name"))
        let purl = nz(opt(entry, "url"))
        html.article[
          #if name != none {
            html.h3[#name]
          }
          #let description = nz(opt(entry, "description"))
          #if description != none {
            html.p[#description]
          }
          #if purl != none {
            html.p[#html.a(href: purl)[#purl]]
          }
          #let dates = date_range(entry)
          #if dates != none {
            html.p[#dates]
          }
        ]
      }
    ]
  }

  // --- References --------------------------------------------------
  #let references = opt(resume, "references")
  #if references != none and type(references) == array and references.len() > 0 {
    html.section[
      #html.h2[References]
      #for entry in references {
        let name = nz(opt(entry, "name"))
        html.article[
          #if name != none {
            html.h3[#name]
          }
          #let reference = nz(opt(entry, "reference"))
          #if reference != none {
            html.p[#reference]
          }
        ]
      }
    ]
  }
]
