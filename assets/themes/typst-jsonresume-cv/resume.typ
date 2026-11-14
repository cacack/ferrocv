#import "base.typ": *

#let getProfile(resume, network) = {
  let profile = none
  // ferrocv vendor patch: `basics` itself is optional per JSON Resume v1.0.0.
  // See VENDORING.md ("optional-field shim").
  let basics = resume.at("basics", default: (:))
  if "profiles" in basics and basics.profiles != none {
    for p in basics.profiles {
      if "network" in p and p.network == network {
        profile = p
        break
      }
    }
  }

  profile
}

// Set data
// ferrocv vendor patch: upstream read from "../../output/resume-data.json" (the Node
// build script copies the user's resume.json there). In ferrocv, the FerrocvWorld
// serves the JSON Resume bytes at the virtual path "/resume.json". See VENDORING.md.
#let r = json("/resume.json")

// ferrocv vendor patch (optional-field shim): JSON Resume v1.0.0 has zero
// required fields, but upstream assumed `meta.language`, `basics.location.city`,
// `basics.location.region`, `basics.email`, and `basics.phone` would always
// be present. Each read is wrapped in `.at(..., default: ...)` so any
// schema-valid document renders. See VENDORING.md.
#let basics = r.at("basics", default: (:))
#let meta = r.at("meta", default: (:))
#let location = basics.at("location", default: (:))

#let lang = meta.at("language", default: "en")
#let name = basics.at("name", default: "")
#let city = location.at("city", default: "")
#let region = location.at("region", default: "")
#let address = if city != "" and region != "" {
  city + ", " + region
} else if city != "" {
  city
} else {
  region
}
#let emailAddress = basics.at("email", default: "")
#let phoneNumber = basics.at("phone", default: "")
#let website = basics.at("url", default: none) //Set to none if you want to hide it
#let githubProfile = none
#let linkedinProfile = none
// ferrocv vendor patch (entry-field shim): a profile's `url` is optional.
#if getProfile(r, "GitHub") != none {
  githubProfile = getProfile(r, "GitHub").at("url", default: none)
}
#if getProfile(r, "LinkedIn") != none {
  linkedinProfile = getProfile(r, "LinkedIn").at("url", default: none)
}

// Configure visibility of sections
#let show_work = true
#let show_projects = true
#let show_education = true
#let show_cert_skills_interests = true

#show: resume.with(
  author: name,
  location: address,
  email: emailAddress,
  language: lang,
  ..if githubProfile != none {
    (github: githubProfile)
  },

  ..if linkedinProfile != none {
    (linkedin: linkedinProfile)
  },

  phone: phoneNumber,
  // ferrocv vendor patch (bundled font): upstream defaults to
  // "Garamond", which is not bundled, so Typst silently fell back to
  // Libertinus Serif. Name the bundled font explicitly. See VENDORING.md.
  font: "Libertinus Serif",

  ..if website != none {
    ( personal-site: website )
  },
)

// ferrocv vendor patch (entry-field shim): every field inside a section
// entry is optional per JSON Resume v1.0.0, but base.typ's section
// builders read some unconditionally. Merge each entry over defaults
// (an entry's own value wins) so a missing field renders empty instead
// of failing the dictionary lookup. See VENDORING.md.
#let with-defaults(key, defaults) = r.at(key, default: ()).map(e => defaults + e)
#let r = r + (
  work: with-defaults("work", (name: "", position: "")),
  projects: with-defaults("projects", (name: "")),
  education: with-defaults("education", (institution: "", studyType: "", area: "")),
  skills: with-defaults("skills", (name: "", keywords: ())),
  certificates: with-defaults("certificates", (name: "")),
  interests: with-defaults("interests", (name: "")),
)

// Section work experience
// ferrocv vendor patch (optional-field shim): top-level `work`/`projects`/
// `education` keys are all optional per JSON Resume v1.0.0, so guard with
// `in r` before reading.
#if show_work and "work" in r and r.work != none and r.work.len() > 0 {
  work(work: r.work, lang: lang)
}
// Section projects
#if show_projects and "projects" in r and r.projects != none and r.projects.len() > 0 {
  projects(projects: r.projects, lang: lang)
}
// Section education
#if show_education and "education" in r and r.education != none and r.education.len() > 0 {
  edu(education: r.education, lang: lang)
}
// Section certificates, skills and interests
#if show_cert_skills_interests and (
  (("certificates" in r and r.certificates != none and r.certificates.len() > 0) or
  ("skills" in r and r.skills != none and r.skills.len() > 0) or
  ("interests" in r and r.interests != none and r.interests.len() > 0))
) {
  cumulativeCertSkillsInterests(
    ..if "certificates" in r and r.certificates != none {
      (certifications: r.certificates)
    },
    ..if "skills" in r and r.skills != none {
      (skills: r.skills)
    },
    ..if "interests" in r and r.interests != none and r.interests.len() > 0 {
      (interests: r.interests)
    },
    lang: lang,
  )
}