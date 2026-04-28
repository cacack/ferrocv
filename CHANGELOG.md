# Changelog

## [0.6.0](https://github.com/cacack/ferrocv/compare/v0.5.0...v0.6.0) (2026-04-28)


### Features

* **cli:** `ferrocv themes install <spec>` subcommand ([bfe9920](https://github.com/cacack/ferrocv/commit/bfe9920309b54901f0cb5e40c6c80cba7cd81dd7))
* **cli:** default --format html to html-minimal theme ([4b01e3d](https://github.com/cacack/ferrocv/commit/4b01e3d24981e9d486d076f94339f70f71edd191)), closes [#64](https://github.com/cacack/ferrocv/issues/64)
* **install:** fetch + extract Typst Universe packages to cache ([380dec0](https://github.com/cacack/ferrocv/commit/380dec0c667e30d380aac474c5631df8b0119824))
* **install:** scenario coverage with a local fixture HTTP server ([1b0f7ae](https://github.com/cacack/ferrocv/commit/1b0f7aebd4583b31eee09dc350ead4ca79ed2e2f))
* **themes:** add basic-resume theme adapter ([b52332c](https://github.com/cacack/ferrocv/commit/b52332c557036068b831a7759bfdcbe8dd69112f)), closes [#37](https://github.com/cacack/ferrocv/issues/37)
* **themes:** add native html-minimal theme source ([8a4e1a7](https://github.com/cacack/ferrocv/commit/8a4e1a72f658bf0d16a39373df39628212553747)), closes [#64](https://github.com/cacack/ferrocv/issues/64)
* **themes:** assert html-minimal semantic-HTML default via tests ([d2d5a57](https://github.com/cacack/ferrocv/commit/d2d5a57a02cbcceb6408f4dfd131418561d6b51f))
* **themes:** register html-minimal in theme registry ([6da0af6](https://github.com/cacack/ferrocv/commit/6da0af67f73621c0c5dd27aa7e7395580b024cef)), closes [#64](https://github.com/cacack/ferrocv/issues/64)
* **themes:** resolve --theme from local filesystem paths ([056de47](https://github.com/cacack/ferrocv/commit/056de470ea77f5f436fe7f17ee351b3d122a33ca))
* **themes:** resolve @preview/... specs from local installer cache ([7dda1fe](https://github.com/cacack/ferrocv/commit/7dda1fef55f7880c70a1bb6e4eb293d6082cd449))


### Bug Fixes

* **test:** use Path::ends_with for cross-platform cache-miss assertion ([96260cd](https://github.com/cacack/ferrocv/commit/96260cd5820a3fbbbea260501f5eb4a98057f406))

## [0.5.0](https://github.com/cacack/ferrocv/compare/v0.4.0...v0.5.0) (2026-04-20)


### Features

* **action:** add setup-ferrocv composite action ([ed6abe9](https://github.com/cacack/ferrocv/commit/ed6abe9f77dd52f84c9deee385a80bc89ba84cec))
* **cli:** default --theme to text-minimal for pdf rendering ([d41f517](https://github.com/cacack/ferrocv/commit/d41f51779e99a59a8fc50e8bfda918cdc57490b7)), closes [#52](https://github.com/cacack/ferrocv/issues/52)


### Bug Fixes

* **action:** prefer shasum when sha256sum is missing (macOS) ([10ec6a7](https://github.com/cacack/ferrocv/commit/10ec6a7f915de52f7b5e1fdd9787bf078065b0cf))
* **action:** use target-named sha256 sidecar filename ([2ced855](https://github.com/cacack/ferrocv/commit/2ced855a270fde93c53404dc5e76c56819516311))

## [0.4.0](https://github.com/cacack/ferrocv/compare/v0.3.0...v0.4.0) (2026-04-20)


### Features

* **cli:** add `themes list` subcommand ([89a29ab](https://github.com/cacack/ferrocv/commit/89a29ab4b76bc71e58da2628d028c5f308a386d6))


### Bug Fixes

* **cli:** exit 2 on `themes list` stdout write failure ([732c595](https://github.com/cacack/ferrocv/commit/732c595bece6d0dd52e2568b948f370237db6cf7))

## [0.3.0](https://github.com/cacack/ferrocv/compare/v0.2.1...v0.3.0) (2026-04-19)


### Features

* **render:** add HTML output via --format html ([5b795a5](https://github.com/cacack/ferrocv/commit/5b795a55f0c61a7061c24b60d8ea789fce76d9ea))
* **render:** add plain text output via --format text ([2321e20](https://github.com/cacack/ferrocv/commit/2321e20bb77d4b4b6059e811df2b8f869f91e431))
* **themes:** add fantastic-cv theme adapter ([6b2ba42](https://github.com/cacack/ferrocv/commit/6b2ba4206ea38cf0eb553bac3ff7fb3cb211c438))
* **themes:** add modern-cv theme adapter ([f654abd](https://github.com/cacack/ferrocv/commit/f654abdf4b8dc92c3190693b7b966d9af71abb75))


### Bug Fixes

* **themes/fantastic-cv:** guard link() against empty URLs ([1efcf1b](https://github.com/cacack/ferrocv/commit/1efcf1b1135e37349be59921c54aea6eb5b452eb))
* **themes/text-minimal:** render all standard JSON Resume sections ([85d8f2f](https://github.com/cacack/ferrocv/commit/85d8f2f71a08119448023be5d1d19daaf4b7ac85))

## [0.2.1](https://github.com/cacack/ferrocv/compare/v0.2.0...v0.2.1) (2026-04-19)


### Bug Fixes

* **cli:** add summary header to validation error output ([9fd5d6e](https://github.com/cacack/ferrocv/commit/9fd5d6eab659c9ff8b2eafe97171b1dc538538a4))
* **theme:** accept schema-valid-but-sparse resumes in typst-jsonresume-cv ([978081f](https://github.com/cacack/ferrocv/commit/978081f498d0d574c04c5635d2da4d2a7d4ef80d))

## [0.2.0](https://github.com/cacack/ferrocv/compare/v0.1.0...v0.2.0) (2026-04-19)


### Features

* **render:** add render subcommand with typst-jsonresume-cv theme ([3882d2a](https://github.com/cacack/ferrocv/commit/3882d2a9bbe37a7ff32fa51f5bad4980914fb11b)), closes [#12](https://github.com/cacack/ferrocv/issues/12) [#13](https://github.com/cacack/ferrocv/issues/13)
* **render:** embed Typst crate for in-process PDF compilation ([c09c12c](https://github.com/cacack/ferrocv/commit/c09c12c32918a97cfa652a88fe2f4c32ede2983e))


### Bug Fixes

* **test:** normalize golden line endings for Windows CI ([d7a01cc](https://github.com/cacack/ferrocv/commit/d7a01cc769270c43f4947efae0053e6f8d7be586))

## 0.1.0 (2026-04-18)


### Features

* add ferrocv validate subcommand ([693907a](https://github.com/cacack/ferrocv/commit/693907ae170e13479155197cebee0c5d8664c30e)), closes [#10](https://github.com/cacack/ferrocv/issues/10)
* bundle JSON Resume v1.0.0 schema in the binary ([0421e93](https://github.com/cacack/ferrocv/commit/0421e93cd6615408116b7970ed7b1b6d79dcebe4)), closes [#9](https://github.com/cacack/ferrocv/issues/9)
