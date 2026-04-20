# Changelog

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
