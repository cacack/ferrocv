# Tailoring guide: one master, many cuts

`ferrocv` is built around a single idea (CONSTITUTION §7): you maintain
**one comprehensive master `resume.json`** — every role, every highlight,
five pages if that's what it takes — and emit **targeted, audience-specific
cuts** from it. A focused two-page "security" resume and a "leadership"
resume are both *projections* of the same master, never separately
maintained files that drift apart.

This guide walks through tagging a master and producing cuts. The example
master it refers to lives at
[`examples/master.resume.json`](../examples/master.resume.json) — open it
alongside this page, or copy it as a starting point.

For the precise schema rules and the reasoning behind them, see
[ADR 0004](adr/0004-audience-tag-schema.md) (tag schema) and
[ADR 0005](adr/0005-projection-surface.md) (CLI surface). This guide is the
how-to; the ADRs are the spec.

## The mental model

Projection is a stage **upstream of rendering**. It reads the master
unmodified and produces a *derived document that is itself valid JSON
Resume*, which then flows into the normal render pipeline:

```text
master resume.json ──▶ [projection] ──▶ derived (valid JSON Resume) ──▶ [render]
```

Because the derived document is ordinary JSON Resume, you can inspect it,
diff two cuts, commit it, or pipe it straight into `render`.

## Two ways to invoke it

There is **one** projection transform behind **two** CLI surfaces:

- **`ferrocv tailor`** runs projection and *stops*, emitting the derived
  document so you can look at it first:

  ```sh
  ferrocv tailor examples/master.resume.json --audience security -o security.json
  ```

  With no `-o`, the derived JSON goes to **stdout** (diagnostics always go
  to stderr), so it composes in a pipe.

- **`ferrocv render`** accepts the same projection flags and projects then
  renders in one shot:

  ```sh
  ferrocv render examples/master.resume.json --audience security -o security.pdf
  ```

These two are equivalent by construction — `render --audience security` is
the same as `ferrocv tailor --audience security | ferrocv render`. Use `tailor` when you
want to eyeball or commit the cut; use `render --audience` for the quick
one-shot. With **no** projection flags, `render` behaves exactly as it
always has — projection is opt-in and inert by default.

## Tagging the master

Audience tags ride under a single `x-ferrocv` object placed *beside* the
content it applies to (never inside the prose — that would be a content
rewrite, which §7 forbids). There are two tagging surfaces.

### Tag a whole entry with `audience`

Any object in a JSON Resume array — a `work`, `volunteer`, `project`,
`skills`, … entry — can carry an `x-ferrocv.audience` list. Here the OWASP
volunteer entry is restricted to the security cut:

```json
{
  "organization": "OWASP Local Chapter",
  "position": "Workshop Lead",
  "highlights": ["Ran quarterly secure-coding workshops", "..."],
  "x-ferrocv": { "audience": ["security"] }
}
```

It will appear under `--audience security` and be dropped under
`--audience leadership`. A dropped entry takes its highlights with it.

### Tag individual bullets with `highlights`

JSON Resume `highlights` are bare strings, so you can't attach a tag to one
directly. Instead, `x-ferrocv.highlights` is an array **index-parallel** to
the entry's `highlights`: entry *i* is the tag list for `highlights[i]`.

```json
{
  "name": "Northwind Platform",
  "highlights": [
    "Led the zero-downtime migration to short-lived workload credentials",
    "Grew the platform team from 4 to 11 engineers",
    "Cut median CI time from 18 to 6 minutes",
    "Drove the SOC 2 Type II audit to a clean report"
  ],
  "x-ferrocv": {
    "audience": ["security", "leadership"],
    "highlights": [
      ["security"],
      ["leadership"],
      [],
      ["security"]
    ]
  }
}
```

Read this as: bullet 0 is for the security cut, bullet 1 for leadership,
bullet 2 for everyone, bullet 3 for security. The entry itself is tagged
for both audiences, so it survives either cut; the per-bullet tags then
decide which highlights show.

### Untagged means universal

The default is **include**. An item with no tag — or with an empty `[]`
tag — is *universal*: kept in every cut. Only an item that is *tagged and
doesn't list the selected audience* is dropped. This makes adoption safe
and incremental: pointing `--audience` at a master you've only partly
tagged never silently drops a whole job for lack of a tag. Tag the few
things that are audience-specific; leave everything else untagged.

> An empty `[]` always means "universal," never "exclude from all." If you
> want to *document* that a bullet is for everyone, `[]` is a fine way to
> say so — it will never invert on you.

## Worked example

Run the security cut against the example master:

```sh
ferrocv tailor examples/master.resume.json --audience security
```

What you get, and why:

| Master content | In `--audience security`? | Reason |
|---|---|---|
| Northwind entry | kept | tagged `["security","leadership"]` |
| └ "Grew the platform team…" bullet | **dropped** | bullet tagged `["leadership"]` only |
| └ "Cut median CI time…" bullet | kept | bullet tag `[]` → universal |
| Cobalt entry | kept | untagged entry → universal |
| └ "Mentored four engineers…" bullet | **dropped** | bullet tagged `["leadership"]` |
| Riverstone entry | kept | fully untagged → universal |
| OWASP volunteer entry | kept | tagged `["security"]` |
| Security skill | kept | tagged `["security"]` |

Now the leadership cut:

```sh
ferrocv tailor examples/master.resume.json --audience leadership
```

The difference: the **OWASP volunteer entry and the Security skill
disappear** (they're tagged security-only), and within Northwind the
security bullets drop while "Grew the platform team…" stays.

## Mechanical filters

Alongside the curated `--audience`, three theme-agnostic filters trim by
rule rather than by tag. They compose with `--audience` (and each other):

- **`--since <YYYY|YYYY-MM|YYYY-MM-DD>`** — drop `work` entries that ended
  before the cutoff; ongoing roles (no `endDate`) are always kept. On the
  example, `--since 2015` drops the Riverstone role (ended 2014).
- **`--max-bullets <N>`** — cap every `highlights` list at the first N
  bullets. It runs **after** `--audience`, so it caps the
  already-curated set.
- **`--redact pii`** — remove `basics.location`, `basics.phone`, and
  `basics.email` from the cut. Identity fields (`name`, `label`,
  `summary`, `url`, `profiles`) are kept. Useful for a resume you post
  publicly.

```sh
# Security cut, last decade only, two bullets max, contact info stripped:
ferrocv tailor examples/master.resume.json \
  --audience security --since 2016 --max-bullets 2 --redact pii -o public-security.json
```

## Tags are stripped from the cut

The `x-ferrocv` metadata is yours, for the tool — it should not travel to a
recruiter. Projection **removes every `x-ferrocv` key it consumed** from the
derived document. Your master keeps its tags untouched; only the cut is
cleaned. (This is enforced: the derived document carries no `x-ferrocv`
anywhere.)

## Cautions

- **`x-ferrocv.highlights` is positional — re-check it after reordering.**
  The tag array is matched to bullets *by index*. If you reorder or insert
  bullets, the tags do not move with them. ferrocv hard-errors if the array
  *length* stops matching, but a length-preserving reorder (swap two
  bullets) leaves the array the right size now pointing at the wrong
  strings — and nothing can detect that. After you reshuffle an entry's
  highlights, re-check its `x-ferrocv.highlights` lines up.

- **Typos in the namespace fail silently.** Unknown `x-` fields are ignored
  by design, so `x-ferovcv` (or `highlght`) drops the whole spec with no
  error — and because untagged content is universal, the resulting cut
  looks plausible but is actually un-tailored. Double-check the spelling is
  exactly `x-ferrocv` with `audience` / `highlights`.

- **`tailor` with no `-o` prints full PII to stdout.** The derived document
  includes everything `--redact` didn't remove. That's fine interactively,
  but prefer `-o <file>` in shared, recorded, or CI contexts.

## See also

- [`examples/master.resume.json`](../examples/master.resume.json) — the
  example master used throughout this guide.
- [ADR 0004](adr/0004-audience-tag-schema.md) — the tag schema and why it
  looks the way it does.
- [ADR 0005](adr/0005-projection-surface.md) — why both `tailor` and
  `render` flags exist over one transform.
- The README [Projection section](../README.md#projection-one-master-many-cuts)
  — the condensed flag reference.
