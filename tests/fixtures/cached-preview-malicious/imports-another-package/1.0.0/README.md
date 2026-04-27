# Stage C malicious-import regression fixture

This fixture mimics a cached `@preview/...` package whose source
includes a forbidden `#import "@preview/cetz:0.2.0": *"` statement.
Stage C resolves `@preview/...` specs *outside* the Typst World by
materializing the cache contents into an `OwnedTheme`, but the World's
own `source()` / `file()` methods still hard-reject `@preview/...`
imports inline (CONSTITUTION §6.1). This fixture is the negative-test
asset that proves the inline-rejection guarantee survived Stage C.

The fixture is hand-authored under ferrocv's MIT-or-Apache-2.0 license;
nothing in here is vendored from the Typst Universe.
