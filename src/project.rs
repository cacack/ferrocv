//! Projection: the mechanical, theme-agnostic selection stage.
//!
//! Projection is a distinct stage *upstream* of rendering
//! (`CONSTITUTION.md` §7): it takes a master JSON Resume document plus a
//! selection spec and produces a **derived document that is itself still
//! valid JSON Resume**, which then flows into the existing render
//! pipeline unchanged. The master is consumed read-only (§1); the
//! transform returns a new [`serde_json::Value`].
//!
//! This module implements two layers of selection:
//!
//! - [`ProjectionSpec::audience`] — *curated*, tag-driven selection
//!   (issue #149, ADR 0004): keep array elements that are untagged or
//!   carry the requested audience under `x-ferrocv.audience`, and within
//!   surviving elements keep the highlights whose index-parallel
//!   `x-ferrocv.highlights` tag matches. The consumed `x-ferrocv` keys
//!   are stripped from the derived document.
//! - [`ProjectionSpec::since`] — drop `work` entries that ended before a
//!   cutoff date; ongoing entries (no `endDate`) are always kept.
//! - [`ProjectionSpec::max_bullets`] — cap every `highlights` array at
//!   the first N entries by position.
//! - [`ProjectionSpec::redact`] — remove named PII fields from `basics`.
//!
//! Selection lives here in Rust, never in themes (§4/§5): a theme only
//! ever sees the already-narrowed document. The mechanical filters
//! (`since`/`max_bullets`/`redact`) do **not** touch `x-ferrocv`; only
//! curated `audience` selection consumes and strips it.
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

/// The projection selection spec: the curated `audience` filter (#149)
/// plus the mechanical `since` / `max_bullets` / `redact` filters (#148).
///
/// An all-`None` spec is a no-op: [`project`] returns the input
/// unchanged (structurally).
///
/// Marked `#[non_exhaustive]` because the struct is *documented to grow*:
/// out-of-crate callers construct it via [`ProjectionSpec::default`] plus
/// field assignment rather than a struct literal, so adding a field later
/// is not a breaking change for them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProjectionSpec {
    /// Curated selection: keep only content tagged for this audience
    /// under `x-ferrocv` (ADR 0004). An array element is kept when it is
    /// untagged (no `x-ferrocv.audience`, or an empty list) or its tag
    /// list contains this audience; it is dropped only when tagged
    /// without it. Within surviving elements, highlights are filtered the
    /// same way against the index-parallel `x-ferrocv.highlights`. The
    /// consumed `x-ferrocv` keys are stripped from the derived document.
    /// `None` runs no curated selection (every element is universal).
    pub audience: Option<String>,
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

    /// Validate the spec's flag-*value* formats without touching a
    /// document.
    ///
    /// This covers only errors detectable from the flags alone — currently
    /// just that `since` (if set) is a usable ISO 8601 date. Callers should
    /// run it *before* reading or validating the input document, so a
    /// malformed flag value surfaces as a usage error rather than being
    /// masked by an unrelated schema failure in the document. [`project`]
    /// also calls it, so the guarantee holds even for callers that skip the
    /// early check.
    ///
    /// It does **not** (and cannot) catch document-structural errors like
    /// [`ProjectionError::HighlightsTagMismatch`], which depend on the
    /// document itself; those still surface from [`project`] even after a
    /// clean `validate()`.
    pub fn validate(&self) -> Result<(), ProjectionError> {
        if let Some(since) = &self.since
            && !is_iso_date(since)
        {
            return Err(ProjectionError::InvalidSince(since.clone()));
        }
        Ok(())
    }
}

/// An error from [`project`].
///
/// Two failure classes, and the CLI maps them to *different* exit codes:
/// [`InvalidSince`](ProjectionError::InvalidSince) is a malformed flag
/// value (a usage error), while
/// [`HighlightsTagMismatch`](ProjectionError::HighlightsTagMismatch) is a
/// defect in the master document (treated like a schema failure).
///
/// Marked `#[non_exhaustive]` because the set of failure classes is
/// *documented to grow* (it went 1→2 with curated selection): out-of-crate
/// callers must include a wildcard arm, so adding a variant later is not a
/// breaking change for them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectionError {
    /// The `--since` value is not a recognizable ISO 8601 date.
    InvalidSince(String),
    /// An entry's `x-ferrocv.highlights` tag array is not aligned with
    /// its `highlights` array — different lengths. ADR 0004 makes this a
    /// hard error rather than a silent pad/truncate, because a positional
    /// array that silently misattributes tags after a reordered or
    /// inserted bullet would ship the wrong cut. Names the offending
    /// entry so the user can fix it.
    HighlightsTagMismatch {
        /// The section the entry lives in (e.g. `"work"`).
        section: String,
        /// The entry's index within that section's array.
        index: usize,
        /// A human-friendly label for the entry — the first present of
        /// `name`, `organization`, `institution`, `title`, or `position`
        /// (see [`entry_label`]) — or `None` if it has none.
        name: Option<String>,
        /// Length of the entry's `highlights` array.
        highlights_len: usize,
        /// Length of the misaligned `x-ferrocv.highlights` array.
        tags_len: usize,
    },
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectionError::InvalidSince(value) => write!(
                f,
                "invalid 'since' value {value:?}: expected an ISO 8601 date \
                 (YYYY, YYYY-MM, or YYYY-MM-DD)"
            ),
            ProjectionError::HighlightsTagMismatch {
                section,
                index,
                name,
                highlights_len,
                tags_len,
            } => {
                let label = match name {
                    Some(name) => format!(" {name:?}"),
                    None => String::new(),
                };
                // Both `section` (a top-level JSON key from the input) and
                // `name` are written via Debug (`{:?}`) so a crafted value
                // with control characters can't inject newlines into the
                // stderr stream that tooling may parse.
                write!(
                    f,
                    "{section:?}[{index}]{label}: x-ferrocv.highlights has {tags_len} \
                     tag(s) but the entry has {highlights_len} highlight(s); they \
                     must be the same length"
                )
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

/// Apply the projection `spec` to `doc`, returning a derived document.
///
/// `doc` is never mutated. The returned [`Value`] is a fresh document
/// with the filters applied in a fixed order — `audience`, then `since`,
/// then `max_bullets`, then `redact` — so composition is deterministic.
///
/// Curated `audience` selection runs **first** so that it precedes the
/// positional `max_bullets` cap ("keep what's relevant, *then* cap"), and
/// so the index-parallel `x-ferrocv.highlights` tags are consumed against
/// the entry's full, un-truncated `highlights` array.
///
/// Returns [`ProjectionError::InvalidSince`] if `spec.since` is malformed,
/// or [`ProjectionError::HighlightsTagMismatch`] if an entry's audience
/// highlight-tags are not aligned with its highlights.
pub fn project(doc: &Value, spec: &ProjectionSpec) -> Result<Value, ProjectionError> {
    // Validate the spec up front, before cloning, so a bad flag value is
    // cheap to reject. Callers are encouraged to call `validate()` even
    // earlier (before reading the document) so usage errors win over
    // document errors; this call keeps the guarantee for those that don't.
    spec.validate()?;

    let mut out = doc.clone();

    if let Some(audience) = &spec.audience {
        apply_audience(&mut out, audience)?;
    }
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

/// Curated, tag-driven selection for `--audience` (ADR 0004).
///
/// For every top-level array section, drop the elements that are *tagged
/// and exclude* `audience`, keeping universal (untagged / empty-tag) and
/// matching elements. Within each surviving element that carries a
/// bare-string `highlights` array, filter the highlights by the
/// index-parallel `x-ferrocv.highlights` tags the same way. Finally strip
/// the consumed `x-ferrocv` key from every surviving element (and from
/// the singleton `basics` object) so the derived document is clean JSON
/// Resume that re-validates (#150) and does not leak the user's
/// audience-targeting topology.
///
/// `basics` is a singleton object, not an array, so it carries no audience
/// tag and its resume fields are kept verbatim (PII suppression is
/// `--redact`'s job, not audience selection — ADR 0004); only its
/// `x-ferrocv` control metadata, if any, is stripped.
///
/// Returns [`ProjectionError::HighlightsTagMismatch`] if an entry's
/// `x-ferrocv.highlights` length differs from its `highlights` length.
fn apply_audience(out: &mut Value, audience: &str) -> Result<(), ProjectionError> {
    let Some(root) = out.as_object_mut() else {
        return Ok(());
    };

    for (section, value) in root.iter_mut() {
        // `basics` is a singleton object: no audience tag, but strip its
        // consumed `x-ferrocv` control metadata so it never leaks.
        if let Some(obj) = value.as_object_mut() {
            obj.remove("x-ferrocv");
            continue;
        }
        let Some(entries) = value.as_array_mut() else {
            continue;
        };

        // Rebuild the section in a single pass that carries each entry's
        // *original* index, so a mismatch error names the entry's position
        // in the master document — not its position after earlier entries
        // were audience-dropped (which `retain`-then-`enumerate` would).
        let mut kept: Vec<Value> = Vec::with_capacity(entries.len());
        for (index, mut entry) in std::mem::take(entries).into_iter().enumerate() {
            if !entry_matches_audience(&entry, audience) {
                continue;
            }
            filter_entry_highlights(&mut entry, audience, section, index)?;
            if let Some(obj) = entry.as_object_mut() {
                obj.remove("x-ferrocv");
            }
            kept.push(entry);
        }
        *entries = kept;
    }

    Ok(())
}

/// Whether an array element is kept for `audience` based on its own
/// `x-ferrocv.audience` tag.
///
/// Per ADR 0004's include-by-default rule, an element is kept when it is
/// *untagged* (no `x-ferrocv.audience`, a non-array value, or an empty
/// array — all "universal") or its tag list contains `audience`. It is
/// dropped only when tagged with a non-empty list that omits `audience`.
fn entry_matches_audience(entry: &Value, audience: &str) -> bool {
    match entry.pointer("/x-ferrocv/audience") {
        Some(Value::Array(tags)) if !tags.is_empty() => {
            tags.iter().any(|t| t.as_str() == Some(audience))
        }
        // Absent, empty, or non-array ⇒ universal ⇒ kept.
        _ => true,
    }
}

/// Filter a surviving entry's `highlights` by its index-parallel
/// `x-ferrocv.highlights` tags for `audience`.
///
/// A highlight is kept when its tag slot is universal (absent / empty /
/// non-array) or contains `audience`. No `x-ferrocv.highlights` key ⇒
/// every highlight is universal ⇒ nothing dropped. A present tag array
/// whose length differs from `highlights` is a hard error (ADR 0004):
/// silently misaligned positional tags would ship the wrong cut.
fn filter_entry_highlights(
    entry: &mut Value,
    audience: &str,
    section: &str,
    index: usize,
) -> Result<(), ProjectionError> {
    // Read the tag array (cloned out) before mutating the entry.
    let tags: Option<Vec<Value>> = entry
        .pointer("/x-ferrocv/highlights")
        .and_then(Value::as_array)
        .cloned();
    let Some(tags) = tags else {
        return Ok(());
    };

    // Resolve the highlights length and the offending-entry label via
    // immutable borrows up front, so the mismatch error (and the mutable
    // retain below) don't overlapping-borrow `entry`.
    let highlights_len = match entry.get("highlights").and_then(Value::as_array) {
        Some(highlights) => highlights.len(),
        // `x-ferrocv.highlights` on an entry with no `highlights` array is
        // meaningless and ignored (ADR 0004) — nothing to align to.
        None => return Ok(()),
    };

    if tags.len() != highlights_len {
        return Err(ProjectionError::HighlightsTagMismatch {
            section: section.to_owned(),
            index,
            name: entry_label(entry),
            highlights_len,
            tags_len: tags.len(),
        });
    }

    let highlights = entry
        .get_mut("highlights")
        .and_then(Value::as_array_mut)
        .expect("highlights array presence just checked above");
    let mut keep = tags.iter().map(|slot| match slot {
        Value::Array(audiences) if !audiences.is_empty() => {
            audiences.iter().any(|a| a.as_str() == Some(audience))
        }
        // Absent (covered by length check), empty, or non-array ⇒ universal.
        _ => true,
    });
    highlights.retain(|_| keep.next().unwrap_or(true));
    Ok(())
}

/// A human-friendly label for an array entry, used in error messages.
/// Tries the common JSON Resume identity fields in priority order.
fn entry_label(entry: &Value) -> Option<String> {
    for key in ["name", "organization", "institution", "title", "position"] {
        if let Some(label) = entry.get(key).and_then(Value::as_str) {
            return Some(label.to_owned());
        }
    }
    None
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
/// untouched; mechanical filters do not consume tags — that is
/// [`apply_audience`]'s job, and it runs first (see [`project`]).
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
/// `YYYY-MM-DD` with all-digit, correctly-zero-padded components, a month
/// in `01`–`12`, and — when a day is present — a day that actually exists
/// in that month and year (leap years included). So `2020-13`,
/// `2020-02-31`, and `2021-02-29` are all rejected, while `2020-02-29` is
/// accepted. This is a real calendar check, not just a range bound, so
/// the CLI's "a malformed value is a usage error" promise holds.
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
    let year: u16 = parts[0].parse().ok()?;
    // Range-check the present month/day components against the calendar.
    if let Some(month) = parts.get(1) {
        let month: u8 = month.parse().ok()?;
        if !(1..=12).contains(&month) {
            return None;
        }
        if let Some(day) = parts.get(2) {
            let day: u8 = day.parse().ok()?;
            if !(1..=days_in_month(year, month)).contains(&day) {
                return None;
            }
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

/// Number of days in a given (Gregorian) month, leap years included.
///
/// `month` must already be range-checked to `1..=12` by the caller.
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        2 => 28,
        _ => unreachable!("month is range-checked to 1..=12 before this call"),
    }
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

    /// A master exercising audience tags at both granularities: a
    /// security-only entry, a leadership-only entry, an untagged entry,
    /// and per-highlight tags index-parallel to the highlights.
    fn audience_master() -> Value {
        json!({
            "work": [
                {
                    "name": "Tagged Corp",
                    "highlights": ["sec bullet", "lead bullet", "shared bullet"],
                    "x-ferrocv": {
                        "audience": ["security", "leadership"],
                        "highlights": [["security"], ["leadership"], []]
                    }
                },
                {
                    "name": "Security Only",
                    "x-ferrocv": { "audience": ["security"] }
                },
                {
                    "name": "Leadership Only",
                    "x-ferrocv": { "audience": ["leadership"] }
                },
                {
                    "name": "Untagged Corp",
                    "highlights": ["keep me"]
                }
            ]
        })
    }

    fn highlights(entry: &Value) -> Vec<String> {
        entry["highlights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_str().unwrap().to_owned())
            .collect()
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
            audience: Some("security".into()),
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
    fn audience_drops_entries_tagged_for_other_audiences() {
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&audience_master(), &spec).unwrap();
        // Tagged Corp (has security), Security Only, and Untagged Corp
        // survive; Leadership Only is dropped.
        assert_eq!(
            work_names(&out),
            vec!["Tagged Corp", "Security Only", "Untagged Corp"]
        );
    }

    #[test]
    fn audience_keeps_untagged_entries_as_universal() {
        // Untagged Corp has no x-ferrocv at all ⇒ universal ⇒ kept for
        // every audience, including one nothing is tagged for.
        let spec = ProjectionSpec {
            audience: Some("nobody-tagged-this".into()),
            ..Default::default()
        };
        let out = project(&audience_master(), &spec).unwrap();
        assert_eq!(work_names(&out), vec!["Untagged Corp"]);
    }

    #[test]
    fn audience_empty_tag_list_is_universal_not_excluded() {
        // An explicit `audience: []` means "for everyone", never
        // "exclude from all" (ADR 0004 pins `[]` to universal).
        let doc = json!({
            "work": [{ "name": "Everyone", "x-ferrocv": { "audience": [] } }]
        });
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        assert_eq!(work_names(&out), vec!["Everyone"]);
    }

    #[test]
    fn audience_filters_highlights_within_surviving_entry() {
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&audience_master(), &spec).unwrap();
        let tagged = &out["work"].as_array().unwrap()[0];
        // "sec bullet" (security) kept; "lead bullet" (leadership) dropped;
        // "shared bullet" ([] ⇒ universal) kept.
        assert_eq!(highlights(tagged), vec!["sec bullet", "shared bullet"]);
    }

    #[test]
    fn audience_filters_non_work_sections_generically() {
        // The audience sweep is not work-specific: every top-level array
        // section is filtered, and surviving entries' highlights too. Here
        // a volunteer entry tagged for leadership is dropped, while a
        // security-tagged one survives with only its security highlights.
        let doc = json!({
            "volunteer": [
                {
                    "organization": "Sec Org",
                    "highlights": ["sec", "lead"],
                    "x-ferrocv": { "audience": ["security"], "highlights": [["security"], ["leadership"]] }
                },
                { "organization": "Lead Org", "x-ferrocv": { "audience": ["leadership"] } }
            ]
        });
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        let vols = out["volunteer"].as_array().unwrap();
        assert_eq!(vols.len(), 1, "leadership-only volunteer entry dropped");
        assert_eq!(vols[0]["organization"], "Sec Org");
        assert_eq!(highlights(&vols[0]), vec!["sec"]);
        assert!(vols[0].get("x-ferrocv").is_none(), "x-ferrocv stripped");
    }

    #[test]
    fn audience_strips_x_ferrocv_from_basics() {
        // `basics` is a singleton object (no audience tag), but its
        // consumed x-ferrocv control metadata must still be stripped so a
        // derived cut never leaks the targeting topology.
        let doc = json!({
            "basics": { "name": "Grace", "x-ferrocv": { "note": "internal" } }
        });
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        assert_eq!(out["basics"]["name"], "Grace", "resume fields kept");
        assert!(
            out["basics"].get("x-ferrocv").is_none(),
            "x-ferrocv stripped from basics"
        );
    }

    #[test]
    fn audience_mismatch_error_reports_original_master_index() {
        // The mismatch error must name the entry's position in the master,
        // not its post-filter position. Here work[0] is audience-dropped,
        // so the misaligned work[1] must still be reported as index 1.
        let doc = json!({
            "work": [
                { "name": "Dropped", "x-ferrocv": { "audience": ["leadership"] } },
                {
                    "name": "Misaligned",
                    "highlights": ["a", "b"],
                    "x-ferrocv": { "highlights": [["security"]] }
                }
            ]
        });
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        match project(&doc, &spec) {
            Err(ProjectionError::HighlightsTagMismatch { index, name, .. }) => {
                assert_eq!(index, 1, "must report the master index, not post-filter");
                assert_eq!(name.as_deref(), Some("Misaligned"));
            }
            other => panic!("expected HighlightsTagMismatch, got {other:?}"),
        }
    }

    #[test]
    fn audience_strips_consumed_x_ferrocv_from_survivors() {
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&audience_master(), &spec).unwrap();
        for entry in out["work"].as_array().unwrap() {
            assert!(
                entry.get("x-ferrocv").is_none(),
                "x-ferrocv must be stripped from the derived document"
            );
        }
    }

    #[test]
    fn audience_runs_before_max_bullets() {
        // Curated selection precedes the positional cap: filter to the
        // security highlights first (["sec bullet", "shared bullet"]),
        // then --max-bullets 1 keeps only the first of those.
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            max_bullets: Some(1),
            ..Default::default()
        };
        let out = project(&audience_master(), &spec).unwrap();
        let tagged = &out["work"].as_array().unwrap()[0];
        assert_eq!(highlights(tagged), vec!["sec bullet"]);
    }

    #[test]
    fn audience_highlights_length_mismatch_is_an_error() {
        let doc = json!({
            "work": [{
                "name": "Misaligned",
                "highlights": ["a", "b", "c"],
                "x-ferrocv": { "highlights": [["security"], ["security"]] }
            }]
        });
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        assert_eq!(
            project(&doc, &spec),
            Err(ProjectionError::HighlightsTagMismatch {
                section: "work".into(),
                index: 0,
                name: Some("Misaligned".into()),
                highlights_len: 3,
                tags_len: 2,
            })
        );
    }

    #[test]
    fn audience_ignores_highlight_tags_on_entry_without_highlights() {
        // x-ferrocv.highlights with nothing to align to is ignored, not a
        // mismatch error (ADR 0004).
        let doc = json!({
            "work": [{ "name": "NoHl", "x-ferrocv": { "highlights": [["security"]] } }]
        });
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let out = project(&doc, &spec).unwrap();
        assert_eq!(work_names(&out), vec!["NoHl"]);
    }

    #[test]
    fn audience_does_not_mutate_input() {
        let doc = audience_master();
        let before = doc.clone();
        let spec = ProjectionSpec {
            audience: Some("security".into()),
            ..Default::default()
        };
        let _ = project(&doc, &spec).unwrap();
        assert_eq!(doc, before, "master must be consumed read-only");
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
    fn is_iso_date_enforces_real_calendar_days() {
        // Impossible days for the given month/year are rejected...
        assert!(!is_iso_date("2020-02-31"), "Feb never has 31 days");
        assert!(!is_iso_date("2021-02-29"), "2021 is not a leap year");
        assert!(!is_iso_date("2020-04-31"), "April has 30 days");
        // ...while genuine dates, including leap day, are accepted.
        assert!(is_iso_date("2020-02-29"), "2020 is a leap year");
        assert!(is_iso_date("2000-02-29"), "2000 is a leap year (÷400)");
        assert!(!is_iso_date("1900-02-29"), "1900 is not a leap year (÷100)");
        assert!(is_iso_date("2021-04-30"));
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
