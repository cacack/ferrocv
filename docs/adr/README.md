# Architecture Decision Records

This directory holds Architecture Decision Records (ADRs) for
`ferrocv` — short documents that capture a single significant design
decision, the context it was made in, and the consequences that follow
from it.

ADRs are for decisions whose *reasoning* needs to outlive the commit
message: choices between viable alternatives, trade-offs that will
otherwise get re-litigated, and the non-goals that fall out. They are
not a substitute for `CONSTITUTION.md` (which records the
non-negotiables) or for inline code comments (which explain local
decisions at the point of code).

## Conventions

- **Filename:** `NNNN-kebab-case-title.md`, zero-padded to four digits,
  numbered sequentially in merge order. Gaps are fine if an ADR is
  withdrawn before merge.
- **Scope:** one decision per ADR. If the discussion splinters into
  two decisions, write two ADRs.
- **Structure:** see the template below. Prose > ceremony; keep it
  short enough that someone will actually read it.
- **Status:** `Proposed` while the ADR is in a PR; `Accepted` once
  merged; `Superseded by NNNN` when replaced by a later ADR. Do not
  delete superseded ADRs — they are the history.
- **Amendments:** substantive changes to an accepted ADR are made by
  writing a new ADR that supersedes it, not by editing the original.
  Typo fixes and link repairs in place are fine.

## Template

```markdown
# NNNN. Title in imperative mood

**Status:** Proposed | Accepted | Superseded by NNNN
**Date:** YYYY-MM-DD

## Context

What is the situation that forces a decision? What constraints apply
(from `CONSTITUTION.md`, related issues, upstream dependencies)? Link
to the issue that prompted this ADR.

## Decision

What did we decide? State it in one or two sentences at the top, then
expand.

## Alternatives considered

Each alternative gets a short paragraph: what it is, why it was
attractive, why we didn't pick it. A decision without alternatives is
a foregone conclusion, not an ADR.

## Consequences

What follows from this decision? Both the good (what problems go away)
and the bad (what we're now committed to, or what we've ruled out).
Include non-goals that fall out of the decision.
```
