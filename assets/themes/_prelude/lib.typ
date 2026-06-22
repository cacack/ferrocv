// _prelude/lib.typ — the shared native-theme contract for ferrocv.
//
// This module makes CONSTITUTION §4's `render(data) -> content` native
// theme contract real. Native themes (e.g. `text-minimal`,
// `html-minimal`) `#import` this prelude and write *layout*, leaving the
// defensive field-reading here. The shared piece is **data access only**:
// layout stays per-theme / per-format — frame-extracted plain text and
// semantic HTML cannot share a single `render`, so this prelude
// deliberately ships no layout primitives.
//
// Themes still receive the document as the virtual file `/resume.json`
// and read it with `json("/resume.json")`; the prelude operates on the
// resulting dictionary. It performs no IO and pulls no `@preview/...`
// package, so render stays fully offline (CONSTITUTION §6.1).
//
// JSON Resume v1.0.0 has zero required fields, so every accessor here
// tolerates missing keys and never errors on a schema-valid document.
//
// First-party ferrocv source (no upstream), redistributable under the
// crate's MIT-or-Apache-2.0 dual license; the sibling `LICENSE` is
// duplicated so the prelude stays self-contained if it is ever extracted
// into its own package.

// --- Optional-field helpers ----------------------------------------

// `opt(d, k)` returns `d.at(k)` if `d` is a dictionary and `k` is
// present, otherwise `none`. Lets per-section code stay readable
// without sprinkling `if "x" in d { ... }` everywhere. If `d` is not a
// dictionary (e.g. `none`, a string, or an array), it returns `none`
// regardless of `k` — never errors on an unexpected shape.
#let opt(d, k) = if type(d) == dictionary and k in d { d.at(k) } else { none }

// `nz(s)` collapses both absent and empty-string values to `none` so
// sections can uniformly check `if value != none`.
#let nz(s) = if s == none or s == "" { none } else { s }

// Join a list of optional strings with `sep`, dropping `none`/empty.
// Used for location ("city, region, country") where any subset of
// components may be missing. Returns the empty string `""` when every
// part is absent — NOT `none`, so callers guard with `!= ""`. (Typst's
// `().join(sep)` yields `none`, which would slip past a `!= ""` guard
// and emit a stray empty element; the explicit `""` keeps the contract
// "always a string".)
#let join_present(parts, sep) = {
  let kept = parts.filter(p => p != none and p != "")
  if kept.len() == 0 { "" } else { kept.join(sep) }
}

// Format a date range. Reads `startDate` and `endDate` from `item`
// (a dictionary); either bound may be missing. An absent `endDate`
// with a present `startDate` becomes "Present". Returns `none` if both
// are absent so the caller can skip the line entirely (guard `!= none`).
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

// --- Normalized section accessors ----------------------------------

// `items(d, key)` returns the value at `key` when it is an array,
// otherwise an empty array `()`. This collapses the
// `if x != none and type(x) == array and x.len() > 0 { ... }` guard
// repeated across every section into a single `for entry in items(...)`
// (or a `.len() > 0` check before emitting a section heading). Absent
// keys and non-array values degrade to `()`; an empty array is returned
// as-is (also length 0, so iteration and `.len()` behave identically).
#let items(d, key) = {
  let v = opt(d, key)
  if v != none and type(v) == array { v } else { () }
}

// --- Extension fields (`x-` namespaces) ----------------------------

// Read a JSON Resume `x-<namespace>` extension field — CONSTITUTION §1's
// only sanctioned extension point — from any dict `d` (the whole
// document or a single section item). `namespace` is the bare name
// WITHOUT the `x-` prefix, so themes never hardcode the literal key
// string. Returns the field value (often itself a dict) or `none` when
// the namespace is absent or `d` is not a dict — never errors (JSON
// Resume has zero required fields).
//
// Worked example — badge a highlight an author tagged for an audience:
//   #let tag = opt(ext(entry, "myorg"), "audience")  // none if absent
//   #if tag != none { /* render a badge, reorder, etc. */ }
//
// Sub-keys are read by chaining `opt` (as above), NOT via dot-notation or
// a second argument — both of these silently return `none`:
//   ext(d, "myorg.audience")  // WRONG: reads the literal key "x-myorg.audience"
//   ext(d, "x-myorg")         // WRONG: double-prefixes to "x-x-myorg"
// (A dedicated sub-key parameter can be added when a second caller needs
// it — CONSTITUTION §5.)
//
// The `ferrocv` namespace is RESERVED for ferrocv's own use — do not read
// or write `x-ferrocv` from a theme. ferrocv's projection (§7) consumes
// and strips its `x-ferrocv` control keys from a derived document
// (`src/project.rs`), so `ext(d, "ferrocv")` yields `none` on projected
// output by design. This accessor is for author-defined namespaces a
// theme chooses to honor, not ferrocv's internal projection metadata.
#let ext(d, namespace) = opt(d, "x-" + namespace)
