//! Cache path resolution for installed Typst Universe packages.
//!
//! Layout per CONSTITUTION.md §6.1 amendment and the research-locked
//! decision (`.prompts/001-41-theme-resolution-research/`):
//!
//! ```text
//! {cache_root}/packages/preview/<name>/<version>/
//! ```
//!
//! where `{cache_root}` is either the user-supplied
//! `FERROCV_CACHE_DIR` environment variable (for tests and power
//! users) or `{dirs::cache_dir()}/ferrocv`.
//!
//! The `ferrocv/` owner prefix under `dirs::cache_dir()` ensures we
//! never share a cache with upstream Typst (which puts its own
//! packages at `{dirs::cache_dir()}/typst/packages/preview/`) — our
//! invariants about what lives under our cache root hold only if we
//! own the root.

use std::path::{Path, PathBuf};

use super::InstallError;

/// Name of the env var that overrides the default cache root.
pub const CACHE_DIR_ENV: &str = "FERROCV_CACHE_DIR";

/// Root directory under which cached packages live.
///
/// If `FERROCV_CACHE_DIR` is set (and non-empty) we use it verbatim;
/// otherwise we fall back to `{dirs::cache_dir()}/ferrocv`.
/// `FERROCV_CACHE_DIR=""` is explicitly rejected rather than silently
/// falling through — an empty env var almost always means a shell
/// script that meant to set it failed.
pub fn cache_root() -> Result<PathBuf, InstallError> {
    if let Ok(value) = std::env::var(CACHE_DIR_ENV) {
        if value.is_empty() {
            return Err(InstallError::CacheDirUnresolved);
        }
        return Ok(PathBuf::from(value));
    }
    dirs::cache_dir()
        .map(|p| p.join("ferrocv"))
        .ok_or(InstallError::CacheDirUnresolved)
}

/// Root under which `@preview/...` packages specifically live.
///
/// `{cache_root}/packages/preview/`. Stage C reads from this exact
/// shape when resolving `@preview/...` specs at render time.
pub fn preview_cache_root() -> Result<PathBuf, InstallError> {
    Ok(cache_root()?.join("packages").join("preview"))
}

/// Full path to a specific cached package.
///
/// `{preview_cache_root}/<name>/<version>/`.
pub fn package_cache_dir(name: &str, version: &str) -> Result<PathBuf, InstallError> {
    Ok(preview_cache_root()?.join(name).join(version))
}

/// Create the parent of a given cache path, returning the parent's
/// `Path` so callers can anchor a temp dir against it.
///
/// Separate function because the mkdir-p logic is duplicated at two
/// call sites (the pipeline and tests) and needs to surface IO
/// failures through [`InstallError::Io`] consistently.
pub fn ensure_parent_exists(final_path: &Path) -> Result<PathBuf, InstallError> {
    let parent = final_path
        .parent()
        .ok_or_else(|| InstallError::Io {
            context: format!(
                "cache path has no parent: {}",
                final_path.display(),
            ),
            source: std::io::Error::other("cache path has no parent"),
        })?
        .to_path_buf();
    std::fs::create_dir_all(&parent).map_err(|source| InstallError::Io {
        context: format!("create cache parent {}", parent.display()),
        source,
    })?;
    Ok(parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `std::env::set_var` / `remove_var` are global process state;
    // serialize cache-dir tests so they do not race when the test
    // runner parallelizes.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env_var<F: FnOnce()>(key: &str, value: Option<&str>, body: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let prior = std::env::var(key).ok();
        // SAFETY: tests are serialized via ENV_LOCK above, and the CI
        // test runner does not spawn threads that read this env var
        // concurrently with this test suite.
        unsafe {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        body();
        unsafe {
            match prior {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn env_var_override_is_honored() {
        with_env_var(CACHE_DIR_ENV, Some("/tmp/ferrocv-test-cache"), || {
            let path = cache_root().expect("explicit override resolves");
            assert_eq!(path, PathBuf::from("/tmp/ferrocv-test-cache"));
        });
    }

    #[test]
    fn empty_env_var_is_rejected() {
        with_env_var(CACHE_DIR_ENV, Some(""), || {
            let err = cache_root()
                .expect_err("empty FERROCV_CACHE_DIR must surface as CacheDirUnresolved");
            assert!(matches!(err, InstallError::CacheDirUnresolved));
        });
    }

    #[test]
    fn preview_cache_root_appends_expected_suffix() {
        with_env_var(CACHE_DIR_ENV, Some("/tmp/ferrocv-test-cache"), || {
            let path = preview_cache_root().expect("resolves");
            assert_eq!(
                path,
                PathBuf::from("/tmp/ferrocv-test-cache/packages/preview")
            );
        });
    }

    #[test]
    fn package_cache_dir_layout_is_stable() {
        with_env_var(CACHE_DIR_ENV, Some("/tmp/ferrocv-test-cache"), || {
            let path = package_cache_dir("basic-resume", "0.2.8").expect("resolves");
            assert_eq!(
                path,
                PathBuf::from("/tmp/ferrocv-test-cache/packages/preview/basic-resume/0.2.8")
            );
        });
    }
}
