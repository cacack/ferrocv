//! Projection: the mechanical, theme-agnostic selection stage.
//!
//! Projection is a distinct stage *upstream* of rendering
//! (`CONSTITUTION.md` §7): it takes a master JSON Resume document plus a
//! selection spec and produces a **derived document that is itself still
//! valid JSON Resume**, which then flows into the existing render
//! pipeline unchanged. The master is consumed read-only (§1); the
//! transform returns a new [`serde_json::Value`].
//!
//! This module implements only the **mechanical** filters (issue #148):
//!
//! - [`ProjectionSpec::since`] — drop `work` entries that ended before a
//!   cutoff date; ongoing entries (no `endDate`) are always kept.
//! - [`ProjectionSpec::max_bullets`] — cap every `highlights` array at
//!   the first N entries by position.
//! - [`ProjectionSpec::redact`] — remove named PII fields from `basics`.
//!
//! Curated, tag-driven `--audience` selection (and the consumption /
//! stripping of `x-ferrocv` tags it entails) is issue #149's job; this
//! stage leaves `x-ferrocv` metadata untouched. Selection lives here in
//! Rust, never in themes (§4/§5): a theme only ever sees the
//! already-narrowed document.
//!
//! Both CLI surfaces — the standalone `tailor` subcommand and the
//! `render` projection flags (ADR 0005) — call [`project`], so the two
//! are equivalent by construction.

use std::fmt;

use serde_json::Value;

/// The PII redaction vocabulary accepted by `--redact`.
///
/// A fixed enum (not a free-form field list) so the CLI can surface it
/// as a `clap` `ValueEnum` and reject unknown values as usage errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactSet {
    /// Remove the standard personally-identifying contact fields from
    /// `basics`: `location`, `phone`, and `email`. Identity fields the
    /// document still needs to be useful (`name`, `label`, `summary`,
    /// `url`, `profiles`) are kept.
    Pii,
}

/// The mechanical projection filters (issue #148).
///
/// An all-`None` spec is a no-op: [`project`] returns the input
/// unchanged (structurally). Curated `--audience` selection (#149) will
/// add its field here without changing the existing semantics.
///
/// Marked `#[non_exhaustive]` because the struct is *documented to grow*
/// (the `--audience` field above): out-of-crate callers construct it via
/// [`ProjectionSpec::default`] plus field assignment rather than a struct
/// literal, so adding a field later is not a breaking change for them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProjectionSpec {
    /// Drop `work` entries that ended before this ISO 8601 date (`YYYY`,
    /// `YYYY-MM`, or `YYYY-MM-DD`). Comparison is granularity-aware: an
    /// entry is dropped only when the latest instant its `endDate` could
    /// denote is strictly before the earliest instant the cutoff could
    /// denote, so coarser dates are treated generously (e.g. an `endDate`
    /// of `"2015"` survives `--since 2015-06`). Ongoing entries (no
    /// `endDate`) are always kept.
    pub since: Option<String>,
    /// Cap every `highlights` array at this many entries (first N by
    /// position). `0` empties them.
    pub max_bullets: Option<usize>,
    /// Redact a named set of PII fields from `basics`.
    pub redact: Option<RedactSet>,
}

impl ProjectionSpec {
    /// True when no filter is set, so projection would be a no-op. The
    /// CLI uses this to keep flagless `render` byte-for-byte unchanged.
    ///
    /// Defined as equality with [`ProjectionSpec::default`] (rather than
    /// a hand-written per-field check) so a future field whose default is
    /// "unset" is automatically included — no risk of a new filter being
    /// silently skipped on the `render` fast path.
    pub fn is_noop(&self) -> bool {
        *self == ProjectionSpec::default()
    }
}

/// An error from [`project`].
///
/// Currently the only failure is a malformed `--since` value; the other
/// filters cannot fail on a document that already schema-validates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    /// The `--since` value is not a recognizable ISO 8601 date.
    InvalidSince(String),
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectionError::InvalidSince(value) => write!(
                f,
                "invalid 'since' value {value:?}: expected an ISO 8601 date \
                 (YYYY, YYYY-MM, or YYYY-MM-DD)"
            ),
        }
    }
}

impl std::error::Error for ProjectionError {}

/// Apply the projection `spec` to `doc`, returning a derived document.
///
/// `doc` is never mutated. The returned [`Value`] is a fresh document
/// with the mechanical filters applied in a fixed order — `since`, then
/// `max_bullets`, then `redact` — so composition is deterministic.
///
/// Returns [`ProjectionError::InvalidSince`] if `spec.since` is set but
/// not a valid ISO 8601 date; the document is validated for that before
/// any work is done.
pub fn project(doc: &Value, spec: &ProjectionSpec) -> Result<Value, ProjectionError> {
    // Validate the one fallible input up front, before cloning, so a bad
    // spec is cheap to reject.
    if let Some(since) = &spec.since
        && !is_iso_date(since)
    {
        return Err(ProjectionError::InvalidSince(since.clone()));
    }

    let mut out = doc.clone();

    if let Some(since) = &spec.since {
        apply_since(&mut out, since);
    }
    if let Some(n) = spec.max_bullets {
        apply_max_bullets(&mut out, n);
    }
    if let Some(redact) = spec.redact {
        apply_redact(&mut out, redact);
    }

    Ok(out)
}

/// Drop `work` entries that ended before `since`.
///
/// Scope is deliberately `work` only (issue #148 decision): "drop older
/// roles" maps to employment history. Ongoing entries (no `endDate`) are
/// always kept. The cutoff test is granularity-aware (see
/// [`kept_by_since`]): an entry is dropped only when the latest instant
/// its `endDate` could denote is strictly before the earliest instant
/// the cutoff could denote, so a coarse `endDate` like `"2015"` is kept
/// against `--since 2015-06` rather than being silently dropped by a
/// naive `"2015" < "2015-06"` string compare.
fn apply_since(out: &mut Value, since: &str) {
    if let Some(work) = out.get_mut("work").and_then(Value::as_array_mut) {
        work.retain(|entry| match entry.get("endDate").and_then(Value::as_str) {
            Some(end) => kept_by_since(end, since),
            // No (or non-string) endDate ⇒ ongoing ⇒ keep.
            None => true,
        });
    }
}

/// Decide whether an entry with end date `end` survives the `--since`
/// cutoff.
///
/// JSON Resume permits `endDate` and the cutoff to each be year-,
/// month-, or day-granular, so a plain lexicographic compare misbehaves
/// across mismatched granularities. We instead compare the *latest*
/// instant `end` could denote against the *earliest* instant `since`
/// could denote: the entry is dropped only when it definitely ended
/// before the cutoff. Ambiguous coarse dates are kept — consistent with
/// projection's include-when-in-doubt bias.
///
/// An `end` value that is not a shape-valid date is kept (we cannot prove
/// it is old); `since` is shape-validated by [`project`] before we get
/// here.
fn kept_by_since(end: &str, since: &str) -> bool {
    match (
        normalize_date(end, Bound::Latest),
        normalize_date(since, Bound::Earliest),
    ) {
        (Some(end_max), Some(since_min)) => end_max >= since_min,
        _ => true,
    }
}

/// Which instant of a partial date to resolve to when normalizing.
#[derive(Debug, Clone, Copy)]
enum Bound {
    /// Fill missing month/day with the start of the period (`01`/`01`).
    Earliest,
    /// Fill missing month/day with the end of the period (`12`/`31`).
    Latest,
}

/// Cap every bare-string `highlights` array at `n` entries (first N).
///
/// Applies to the stock JSON Resume v1.0.0 sections that carry a
/// bare-string `highlights` array — `work`, `volunteer`, `projects`.
/// The index-parallel `x-ferrocv.highlights` tag array (an array *of
/// arrays*, nested under `x-ferrocv`) is a sibling key and is left
/// untouched; mechanical filters do not consume tags (#149's job).
fn apply_max_bullets(out: &mut Value, n: usize) {
    for section in ["work", "volunteer", "projects"] {
        if let Some(entries) = out.get_mut(section).and_then(Value::as_array_mut) {
            for entry in entries {
                if let Some(highlights) = entry.get_mut("highlights").and_then(Value::as_array_mut)
                {
                    highlights.truncate(n);
                }
            }
        }
    }
}

/// Remove the redaction set's fields from `basics`.
fn apply_redact(out: &mut Value, redact: RedactSet) {
    let RedactSet::Pii = redact;
    if let Some(basics) = out.get_mut("basics").and_then(Value::as_object_mut) {
        for field in ["location", "phone", "email"] {
            basics.remove(field);
        }
    }
}

/// True if `s` is an ISO 8601 date at year, month, or day granularity:
/// `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`.
///
/// Used by [`project`] to turn a malformed `--since` value into a usage
/// error rather than a silent no-match. Delegates to [`normalize_date`],
/// so it enforces the same range checks (month `01`–`12`, day `01`–`31`).
fn is_iso_date(s: &str) -> bool {
    normalize_date(s, Bound::Earliest).is_some()
}

/// Validate an ISO 8601 date shape and normalize it to a full
/// `YYYY-MM-DD` string, filling absent month/day components per `bound`.
///
/// Returns `None` for anything that is not `YYYY`, `YYYY-MM`, or
/// `YYYY-MM-DD` with all-digit, correctly-zero-padded components and a
/// month in `01`–`12` / day in `01`–`31`. The range check is coarse — it
/// is not a full calendar check, so `2020-02-31` (a non-existent day that
/// is still within `01`–`31`) is accepted; it exists to reject obvious
/// garbage like `2020-13` while keeping the implementation simple.
fn normalize_date(s: &str, bound: Bound) -> Option<String> {
    let parts: Vec<&str> = s.split('-').collect();
    let widths = [4usize, 2, 2];
    // At least the year, at most year-month-day.
    if parts.is_empty() || parts.len() > widths.len() {
        return None;
    }
    for (part, &width) in parts.iter().zip(widths.iter()) {
        if part.len() != width || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    // Range-check the present month/day components.
    if let Some(month) = parts.get(1) {
        let month: u8 = month.parse().ok()?;
        if !(1..=12).contains(&month) {
            return None;
        }
    }
    if let Some(day) = parts.get(2) {
        let day: u8 = day.parse().ok()?;
        if !(1..=31).contains(&day) {
            return None;
        }
    }

    let (fill_month, fill_day) = match bound {
        Bound::Earliest => ("01", "01"),
        Bound::Latest => ("12", "31"),
    };
    let year = parts[0];
    let month = parts.get(1).copied().unwrap_or(fill_month);
    let day = parts.get(2).copied().unwrap_or(fill_day);
    Some(format!("{year}-{month}-{day}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn master() -> Value {
        json!({
            "basics": {
                "name": "Grace Hopper",
                "label": "Engineer",
                "email": "grace@example.com",
                "phone": "+1-555-0100",
                "url": "https://example.com/grace",
                "location": { "city": "Arlington" }
            },
            "work": [
                {
                    "name": "Current Corp",
                    "startDate": "2019-02-01",
                    "highlights": ["a", "b", "c", "d"],
                    "x-ferrocv": { "audience": ["security"] }
                },
                { "name": "Mid Corp", "startDate": "2015-03-01", "endDate": "2018-12-31" },
                { "name": "Old Corp", "startDate": "2003-06-01", "endDate": "2005-06-30" }
            ],
            "volunteer": [
                { "organization": "Mentors", "highlights": ["x", "y", "z"] }
            ]
        })
    }

    fn work_names(doc: &Value) -> Vec<String> {
        doc["work"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn noop_spec_returns_input_unchanged() {
        let doc = master();
        let spec = ProjectionSpec::default();
        assert!(spec.is_noop());
        assert_eq!(project(&doc, &spec).unwrap(), doc);
    }

    #[test]
    fn project_does_not_mutate_input() {
        let doc = master();
        let before = doc.clone();
        let spec = ProjectionSpec {
            since: Some("2015".into()),
            max_bullets: Some(1),
            redact: Some(RedactSet::Pii),
        };
        let _ = project(&doc, &spec).unwrap();
        assert_eq!(doc, before, "master must be consumed read-only");
    }

    #[test]
    fn since_drops_old_keeps_recent_and_ongoing() {
        let spec = ProjectionSpec {
            since: Some("2015".into()),
            ..Default::default()
        };
        let out = project(&master(), &spec).unwrap();
        assert_eq!(work_names(&out), vec!["Current Corp", "Mid Corp"]);
    }

    #[test]
    fn since_boundary_keeps_entry_ending_exactly_at_cutoff() {
        // endDate "2015" is not strictly before "2015" ⇒ kept.
        let doc = json!({ "work": [{ "name": "Edge", "endDate": "2015" }] });
        let spec = ProjectionSpec {
            since: Some("2015".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        assert_eq!(work_names(&out), vec!["Edge"]);
    }

    #[test]
    fn since_is_granularity_aware() {
        // A coarse year-only endDate is kept against a month-precision
        // cutoff: "2015" could mean any month of 2015, so we keep it
        // rather than dropping it the way a naive "2015" < "2015-06"
        // lexicographic compare would.
        let doc = json!({
            "work": [
                { "name": "YearOnly", "endDate": "2015" },
                { "name": "EarlyMonth", "endDate": "2015-05" },
                { "name": "LateMonth", "endDate": "2015-07" }
            ]
        });
        let spec = ProjectionSpec {
            since: Some("2015-06".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        // YearOnly kept (ambiguous), EarlyMonth dropped (definitely
        // before June), LateMonth kept.
        assert_eq!(work_names(&out), vec!["YearOnly", "LateMonth"]);
    }

    #[test]
    fn since_keeps_entry_with_unparsable_end_date() {
        // A non-date endDate cannot be proven old ⇒ kept.
        let doc = json!({ "work": [{ "name": "Weird", "endDate": "present" }] });
        let spec = ProjectionSpec {
            since: Some("2015".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        assert_eq!(work_names(&out), vec!["Weird"]);
    }

    #[test]
    fn max_bullets_caps_work_and_volunteer() {
        let spec = ProjectionSpec {
            max_bullets: Some(2),
            ..Default::default()
        };
        let out = project(&master(), &spec).unwrap();
        let current = &out["work"].as_array().unwrap()[0];
        assert_eq!(
            current["highlights"].as_array().unwrap().as_slice(),
            &[json!("a"), json!("b")]
        );
        assert_eq!(
            out["volunteer"][0]["highlights"].as_array().unwrap().len(),
            2
        );
    }

    #[test]
    fn max_bullets_leaves_x_ferrocv_untouched() {
        let spec = ProjectionSpec {
            max_bullets: Some(1),
            ..Default::default()
        };
        let out = project(&master(), &spec).unwrap();
        assert!(out["work"][0].get("x-ferrocv").is_some());
    }

    #[test]
    fn redact_pii_removes_contact_fields_keeps_identity() {
        let spec = ProjectionSpec {
            redact: Some(RedactSet::Pii),
            ..Default::default()
        };
        let out = project(&master(), &spec).unwrap();
        let basics = out["basics"].as_object().unwrap();
        assert!(!basics.contains_key("location"));
        assert!(!basics.contains_key("phone"));
        assert!(!basics.contains_key("email"));
        assert!(basics.contains_key("name"));
        assert!(basics.contains_key("label"));
        assert!(basics.contains_key("url"));
    }

    #[test]
    fn invalid_since_is_rejected() {
        let spec = ProjectionSpec {
            since: Some("banana".into()),
            ..Default::default()
        };
        assert_eq!(
            project(&master(), &spec),
            Err(ProjectionError::InvalidSince("banana".into()))
        );
    }

    #[test]
    fn is_iso_date_accepts_valid_granularities() {
        assert!(is_iso_date("2020"));
        assert!(is_iso_date("2020-05"));
        assert!(is_iso_date("2020-05-17"));
    }

    #[test]
    fn is_iso_date_rejects_garbage() {
        assert!(!is_iso_date("banana"));
        assert!(!is_iso_date("20"));
        assert!(!is_iso_date("2020-5"));
        assert!(!is_iso_date("2020-05-17-01"));
        assert!(!is_iso_date(""));
        assert!(!is_iso_date("2020-"));
    }

    #[test]
    fn is_iso_date_rejects_out_of_range_components() {
        // Shape-valid but calendar-out-of-range values are rejected so
        // the CLI's "malformed value is a usage error" promise holds.
        assert!(!is_iso_date("2020-13"));
        assert!(!is_iso_date("2020-00"));
        assert!(!is_iso_date("2020-05-32"));
        assert!(!is_iso_date("2020-05-00"));
    }

    #[test]
    fn calendar_invalid_since_is_rejected() {
        let spec = ProjectionSpec {
            since: Some("2020-13".into()),
            ..Default::default()
        };
        assert_eq!(
            project(&master(), &spec),
            Err(ProjectionError::InvalidSince("2020-13".into()))
        );
    }

    #[test]
    fn normalize_date_fills_bounds() {
        assert_eq!(
            normalize_date("2015", Bound::Earliest).as_deref(),
            Some("2015-01-01")
        );
        assert_eq!(
            normalize_date("2015", Bound::Latest).as_deref(),
            Some("2015-12-31")
        );
        assert_eq!(
            normalize_date("2015-06", Bound::Latest).as_deref(),
            Some("2015-06-31")
        );
        assert_eq!(
            normalize_date("2015-06-15", Bound::Earliest).as_deref(),
            Some("2015-06-15")
        );
    }
}
