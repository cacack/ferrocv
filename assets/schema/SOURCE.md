# JSON Resume schema provenance

| Field | Value |
| --- | --- |
| Upstream URL | <https://raw.githubusercontent.com/jsonresume/resume-schema/v1.0.0/schema.json> |
| Git tag | `v1.0.0` |
| Commit SHA | `7095651fbbb593d2c5dc2db3095412b170d74d2e` |
| Retrieved on | 2026-04-18 |

Bumping the embedded schema is an intentional release action that requires
constitutional-style review (see issue #9 rationale and `CONSTITUTION.md`
sections 5 and 6.1): `ferrocv` makes no network calls at validate or render
time, so the schema is frozen per release rather than fetched live.
