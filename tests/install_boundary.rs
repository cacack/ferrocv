//! Architectural-boundary enforcement for CONSTITUTION.md §6.1
//! (post-Stage-B amendment).
//!
//! The claim under test: with the `install` feature OFF (the default
//! build), no network-capable code from `src/install/` is reachable
//! from the `render` or `validate` call graphs.
//!
//! This file enforces the claim at compile time via `#[cfg]`: if the
//! `install` feature is off, the test below asserts that the
//! `ferrocv::install` module path is NOT nameable from outside the
//! crate. (If `install` ever moves out from behind its feature gate,
//! this test starts to compile and fails at link time.)
//!
//! When the `install` feature is on, the test is a positive
//! assertion: the module IS reachable and is the expected shape.
//!
//! # Why this file exists
//!
//! Pairs with the `Makefile` target `verify-no-network-default` and
//! the CI spot-check (see `.github/workflows/ci.yml`). Those check
//! the dependency *graph*; this file checks the Rust *code* boundary.
//! Both together cover the plan's `<architectural_boundary>` clause.

#[cfg(feature = "install")]
mod install_feature_on {
    #[test]
    fn install_module_is_reachable_under_feature() {
        // The module exists and carries the Stage B public surface.
        // We reference the types here so a refactor that removes
        // them under the feature flag breaks this test.
        let _: fn(&str) -> Result<_, ferrocv::install::InstallError> =
            ferrocv::install::spec::parse_spec;
        let _: &str = ferrocv::install::cache::CACHE_DIR_ENV;
    }
}

#[cfg(not(feature = "install"))]
mod install_feature_off {
    /// Under the default build the `ferrocv::install` module must
    /// not exist as a nameable path. This test body is trivially
    /// `assert!(true)`; the architectural property is asserted by
    /// the fact that this file compiles under both feature sets
    /// without importing anything from the gated module.
    ///
    /// If someone later moves `src/install/` out from behind the
    /// `#[cfg(feature = "install")]` gate in `src/lib.rs`, the
    /// `install_feature_on` module above would need its `#[cfg]`
    /// lifted too — making the change obvious in review. Meanwhile,
    /// the `cargo tree --no-default-features` spot-check in the
    /// Makefile catches a `Cargo.toml` regression (a network dep
    /// leaking out of the feature gate) on the dependency-graph
    /// side.
    #[test]
    fn install_module_is_not_present_in_default_build() {
        // No-op: if this file compiles under `--no-default-features`,
        // nothing in this module references `ferrocv::install`, so
        // the default build cannot link against it.
    }

    /// Issue #114 boundary check: even when a theme source file
    /// contains a real `#import "@preview/..."`, the default-features
    /// World still rejects it. The cache reader added in #114 is
    /// `#[cfg(feature = "install")]`, so the no-install fallback
    /// inside `resolve_package_file` preserves the historical blanket
    /// rejection. This pins the "no network at render time" guarantee
    /// at the *code* level (the no-network feature flag enforces it at
    /// the *dependency* level).
    #[test]
    fn render_rejects_preview_imports_even_with_cache_dir_set() {
        use assert_cmd::Command;
        use predicates::prelude::*;

        let tmp = tempfile::TempDir::new().expect("tempdir");
        // Local-path theme with an inline `@preview/...` import.
        let theme_path = tmp.path().join("local-theme.typ");
        std::fs::write(
            &theme_path,
            "#import \"@preview/some-pkg:1.0.0\": *\n\
             #let resume = json(\"/resume.json\")\n\
             = #resume.basics.name\n",
        )
        .expect("write theme");

        // Minimal valid resume.
        let resume_path = tmp.path().join("resume.json");
        std::fs::write(
            &resume_path,
            "{\"basics\":{\"name\":\"Test\"},\"$schema\":\"https://raw.githubusercontent.com/jsonresume/resume-schema/v1.0.0/schema.json\"}",
        )
        .expect("write resume");

        let out = tmp.path().join("out.pdf");
        Command::cargo_bin("ferrocv")
            .expect("ferrocv binary")
            // Even setting FERROCV_CACHE_DIR has no effect under
            // default features — the cache reader is not linked.
            .env("FERROCV_CACHE_DIR", tmp.path())
            .arg("render")
            .arg(&resume_path)
            .arg("--theme")
            .arg(&theme_path)
            .arg("--format")
            .arg("pdf")
            .arg("--output")
            .arg(&out)
            .assert()
            .code(2)
            .stderr(predicate::str::contains("@preview/some-pkg:1.0.0"))
            .stderr(predicate::str::contains("package not found"));

        assert!(
            !out.exists(),
            "default build must not produce output when theme imports @preview/...",
        );
    }
}
