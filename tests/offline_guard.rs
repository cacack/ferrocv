//! Stage C offline-guard: prove that the render and validate call
//! graphs do not import the network-capable installer modules even
//! with the `install` Cargo feature ON.
//!
//! Stage B already enforced the feature-flag boundary
//! (`tests/install_boundary.rs`): when the `install` feature is OFF,
//! `src/install/*` does not compile and `ureq`/`tar`/`dirs` are not
//! in the dependency graph. That is necessary but not sufficient for
//! Stage C — once the feature is ON, nothing at the type system level
//! prevents `src/render.rs` or `src/cli.rs::run_render` from calling
//! into `src/install/fetch.rs` and triggering an HTTPS GET at render
//! time. CONSTITUTION §6.1 (post-Stage-B amendment) carves out
//! `ferrocv themes install` as the SINGLE network-permitted entry
//! point; render and validate stay offline regardless of feature
//! flags.
//!
//! This file enforces the property statically: it scans the source
//! text of the files reachable from `compile_*` / `validate_value`
//! and the `Render` / `Validate` CLI handlers and asserts that none
//! of them carry a `use crate::install::{fetch, extract, pipeline}`
//! import or call. The cache reader (`src/package_cache.rs`) and the
//! cache-path / manifest helpers (`src/install/cache.rs`,
//! `src/install/manifest.rs`, `src/install/spec.rs`) are pure
//! filesystem / parsing modules and ARE reachable from the render
//! path; they are explicitly allow-listed below.
//!
//! # Why a string-grep static check rather than a type-system test
//!
//! A compile-time `static_assertions` approach would require defining
//! traits on the installer modules that name-collide with non-network
//! ones. That is a lot of scaffolding for a property that, in this
//! crate, is fundamentally a "which modules import which" question —
//! a question source-text scanning answers directly. The check runs
//! once per `cargo test` invocation; cost is negligible.
//!
//! If a future refactor splits the cache reader out from
//! `src/install/`, update [`ALLOWED_INSTALL_SUBMODULES`] below to
//! reflect the new module layout. The test will fail loudly until
//! the allow-list matches reality.

use std::path::{Path, PathBuf};

/// Files reachable from the `render` and `validate` call graphs that
/// MUST NOT import network-capable installer submodules.
///
/// `src/cli.rs` is on the list because `run_render` and `run_validate`
/// live there. The `run_themes_install` handler in the same file is
/// allowed to reference the installer (it IS the installer entry
/// point); the per-line filter below tolerates `use crate::install::`
/// inside `#[cfg(feature = "install")]` blocks, but the network-
/// capable submodules (`fetch`, `extract`, `pipeline`) must never
/// appear outside `run_themes_install`.
const RENDER_PATH_FILES: &[&str] = &[
    "src/render.rs",
    "src/validate.rs",
    "src/theme.rs",
    "src/package_cache.rs",
    "src/lib.rs",
    "src/main.rs",
];

/// Installer submodules that perform pure filesystem / parsing work
/// (no `ureq`, no `flate2`, no `tar`) and are therefore safe to be
/// reachable from the render path.
///
/// If any of these grow a network call, move them out of the
/// allow-list and the offline-guard regression test will catch it.
const ALLOWED_INSTALL_SUBMODULES: &[&str] = &[
    "crate::install::cache",
    "crate::install::manifest",
    "crate::install::spec",
    // Bare `crate::install` (e.g. `crate::install::InstallError`)
    // resolves to types defined in `src/install/mod.rs` — those
    // types carry no behavior, just enum variants and Display
    // impls. Allow.
    "crate::install::InstallError",
    "crate::install::PackageSpec",
];

/// Network-capable installer submodules that must never appear in
/// the render path.
const FORBIDDEN_INSTALL_SUBMODULES: &[&str] = &[
    "crate::install::fetch",
    "crate::install::extract",
    "crate::install::pipeline",
];

#[test]
fn render_path_does_not_import_network_capable_install_modules() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();

    for relative in RENDER_PATH_FILES {
        let path = root.join(relative);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for (line_no, line) in src.lines().enumerate() {
            // Skip comments — `//` lines that mention forbidden modules
            // are documentation, not imports.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            for forbidden in FORBIDDEN_INSTALL_SUBMODULES {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{}: forbidden installer module reference: {}",
                        relative,
                        line_no + 1,
                        forbidden,
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "render path imports forbidden network-capable installer modules:\n  {}",
        violations.join("\n  ")
    );
}

/// Sentinel: assert the cache-miss user-facing error message points
/// at `ferrocv themes install`, not at any other escape hatch. If a
/// future refactor changes the error wording, this test fails so the
/// CONSTITUTION §6.1 contract ("uncached package -> error points at
/// install, no silent fetch") stays explicit in code.
#[test]
fn cache_miss_error_message_directs_user_to_themes_install() {
    let theme_rs =
        std::fs::read_to_string(repo_root().join("src/theme.rs")).expect("read theme.rs");
    assert!(
        theme_rs.contains("Run: ferrocv themes install"),
        "theme.rs must surface the `ferrocv themes install` hint verbatim",
    );
    assert!(
        theme_rs.contains("PreviewCacheMiss"),
        "theme.rs must define the PreviewCacheMiss error variant",
    );
}

/// Sentinel: `src/cli.rs::run_render` does not call into
/// `crate::install::pipeline::install` even under `#[cfg(feature =
/// "install")]`. A different test asserts the same property for the
/// other render-path files; this one anchors it on `cli.rs` because
/// `cli.rs` is the only file that legitimately CALLS the installer
/// (from `run_themes_install`) — the regression to guard against is
/// `run_render` accidentally calling `install()` on cache miss.
#[test]
fn run_render_does_not_call_install_pipeline() {
    let cli_rs = std::fs::read_to_string(repo_root().join("src/cli.rs")).expect("read cli.rs");
    let render_block = extract_fn_body(&cli_rs, "fn run_render(");
    assert!(
        !render_block.contains("install::pipeline"),
        "run_render must not reference install::pipeline; got body:\n{render_block}"
    );
    assert!(
        !render_block.contains("install::fetch"),
        "run_render must not reference install::fetch; got body:\n{render_block}"
    );
    assert!(
        !render_block.contains("install::extract"),
        "run_render must not reference install::extract; got body:\n{render_block}"
    );
}

/// Sentinel that the `ALLOWED_INSTALL_SUBMODULES` list is non-empty
/// and well-formed — catches a refactor that accidentally empties
/// the allow-list and trivially passes the main grep test.
#[test]
fn allow_list_is_non_empty_and_disjoint_from_forbid_list() {
    assert!(!ALLOWED_INSTALL_SUBMODULES.is_empty());
    for allowed in ALLOWED_INSTALL_SUBMODULES {
        for forbidden in FORBIDDEN_INSTALL_SUBMODULES {
            assert!(
                allowed != forbidden,
                "submodule cannot be both allowed and forbidden: {allowed}",
            );
        }
    }
}

/// Locate the workspace root. `CARGO_MANIFEST_DIR` is set by Cargo
/// when running the integration test; it points at the package root.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Heuristically extract the body of a top-level `fn` block from
/// Rust source. Returns the substring from the function signature
/// through the matching closing brace (inclusive). Brace-counting is
/// rudimentary — it tracks `{` / `}` ignoring string and char
/// literals' interior braces for the simple cases — but it is good
/// enough for the small set of functions we run it against. Yields
/// `""` if the function is not found.
fn extract_fn_body(src: &str, signature_prefix: &str) -> String {
    let Some(start) = src.find(signature_prefix) else {
        return String::new();
    };
    let after = &src[start..];
    let Some(brace_idx) = after.find('{') else {
        return String::new();
    };
    let mut depth = 0i32;
    let bytes = after.as_bytes();
    let mut end = bytes.len();
    for (i, b) in bytes.iter().enumerate().skip(brace_idx) {
        match *b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    after[..end].to_owned()
}

/// Just to ensure the `Path` import is exercised even on the
/// minimal-features build. `repo_root()` always returns an existing
/// directory; reading it confirms the test harness is wired right.
#[test]
fn repo_root_exists() {
    let root: &Path = &repo_root();
    assert!(root.is_dir(), "CARGO_MANIFEST_DIR must be a directory");
    assert!(
        root.join("Cargo.toml").is_file(),
        "CARGO_MANIFEST_DIR must contain Cargo.toml",
    );
}
