# Stage C cache-hit fixture (not vendored upstream)

This directory mimics the on-disk shape that
`ferrocv themes install @preview/basic-resume:0.2.8` would write into
the cache. It is **not** a copy of the upstream `basic-resume` Typst
Universe package — it is a hand-authored test asset under the
`ferrocv` MIT-or-Apache-2.0 license.

The fixture exists because the real upstream `basic-resume` package
imports `@preview/scienceicons` at compile time. `FerrocvWorld` rejects
every `@preview/...` import per CONSTITUTION §6.1, so the upstream
package would fail to compile under our offline World even after a
successful cache hit. This fixture deliberately has zero `@preview/...`
imports so the cache-hit scenario test can assert on a clean PDF
output.

The directory layout (`typst.toml`, `src/lib.typ`, `src/helpers.typ`)
mirrors the multi-file shape Stage C's resolver
(`src/package_cache.rs::resolve_preview_spec_from_cache`) walks: the
test exercises both the entrypoint registration and a sibling-file
import.

See also the malicious-import fixture under
`tests/fixtures/cached-preview-malicious/` — that one DOES carry a
`@preview/...` import and is used to regression-test the World-layer
rejection on theme source.
