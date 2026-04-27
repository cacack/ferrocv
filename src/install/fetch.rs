//! HTTPS fetch of Typst Universe tarballs via `ureq`.
//!
//! Single-responsibility: given a URL, return the response body as
//! bytes or an [`InstallError`]. No retry, no caching, no streaming
//! — a resume-theme tarball is typically 5–50 KB so a
//! single-in-memory buffer is fine. CONSTITUTION.md §5
//! ("simple now, iterate later") calls the narrower solution here.
//!
//! TLS: `ureq`'s default features are rustls + ring. We do NOT enable
//! `native-tls` or anything that would pull in `openssl`.
//!
//! Timeouts: a single 30s wall-clock timeout. Typst Universe tarballs
//! are small and Azure-Blob-backed; if we haven't finished in 30s
//! something is wrong.

use std::io::Read;
use std::time::Duration;

use super::{InstallError, PackageSpec};

/// Default registry URL prefix. Override via the
/// `FERROCV_REGISTRY_URL` env var in tests to point at a local
/// fixture server.
pub const DEFAULT_REGISTRY: &str = "https://packages.typst.org/preview";

/// Name of the env var that overrides [`DEFAULT_REGISTRY`]. Exists
/// solely so integration tests can point the fetcher at a local
/// `TcpListener` instead of reaching out to the real Typst Universe.
/// Never documented in user-facing help text: the flag is a
/// test-only escape hatch, not a config knob.
pub const REGISTRY_URL_ENV: &str = "FERROCV_REGISTRY_URL";

/// Default wall-clock timeout for a full fetch.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum response body size we will read before erroring out.
///
/// Resume themes are 5–50 KB; even the heaviest Typst Universe
/// packages (`cetz` at 74 KB compressed, `polylux` at ~150 KB) are
/// well under 10 MB. Capping at 16 MB is comfortable for outliers
/// while still bounding memory for a malicious server that streams
/// random bytes indefinitely.
pub const MAX_TARBALL_BYTES: u64 = 16 * 1024 * 1024;

/// Root URL the fetcher uses for this process invocation.
///
/// Honors `FERROCV_REGISTRY_URL` when set (test-only escape hatch)
/// and falls back to [`DEFAULT_REGISTRY`] otherwise. Treated as a
/// URL prefix that the spec's `<name>-<version>.tar.gz` appends to.
fn registry_root() -> String {
    match std::env::var(REGISTRY_URL_ENV) {
        Ok(v) if !v.is_empty() => v,
        _ => DEFAULT_REGISTRY.to_owned(),
    }
}

/// Construct the canonical tarball URL for a spec against the
/// configured registry root (see [`registry_root`]).
///
/// The default registry root bakes in the `preview` namespace, so this
/// function only interpolates `name` and `version`. The
/// `debug_assert_eq!` makes that coupling explicit: if a future change
/// loosens [`crate::install::spec::parse_spec`] to accept other
/// namespaces (e.g. `@local`), this assertion fires under
/// `cargo test` rather than silently fetching the wrong URL. Release
/// builds drop the assertion.
pub fn tarball_url(spec: &PackageSpec) -> String {
    debug_assert_eq!(
        spec.namespace, "preview",
        "tarball_url currently only supports the @preview namespace; \
         widen DEFAULT_REGISTRY before relaxing parse_spec"
    );
    let root = registry_root();
    let root = root.trim_end_matches('/');
    format!(
        "{root}/{name}-{version}.tar.gz",
        name = spec.name,
        version = spec.version,
    )
}

/// Fetch the tarball for `spec` from the default registry.
pub fn fetch_tarball(spec: &PackageSpec) -> Result<Vec<u8>, InstallError> {
    fetch_tarball_from(&tarball_url(spec))
}

/// Fetch a tarball from an arbitrary URL. Used by tests to point at a
/// local fixture server; production code goes through
/// [`fetch_tarball`].
pub fn fetch_tarball_from(url: &str) -> Result<Vec<u8>, InstallError> {
    // Turn off `http_status_as_error` so 4xx/5xx responses come back
    // as successful `Response` values we can inspect; otherwise the
    // status-specific `InstallError::HttpStatus` branch would never
    // fire (ureq would translate 404s into a generic
    // `ureq::Error::StatusCode`).
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent.get(url).call().map_err(|e| InstallError::Http {
        url: url.to_owned(),
        reason: e.to_string(),
    })?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(InstallError::HttpStatus {
            url: url.to_owned(),
            status,
        });
    }
    let mut reader = response
        .body_mut()
        .as_reader()
        .take(MAX_TARBALL_BYTES.saturating_add(1));
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|source| InstallError::Io {
            context: format!("read tarball body from {url}"),
            source,
        })?;
    if (buf.len() as u64) > MAX_TARBALL_BYTES {
        return Err(InstallError::Io {
            context: format!("tarball body exceeded {MAX_TARBALL_BYTES} bytes"),
            source: std::io::Error::other("tarball body too large"),
        });
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::spec::parse_spec;

    #[test]
    fn url_matches_registry_convention() {
        // Guard against leaky env state from other tests that set the
        // registry override. We snapshot, clear, assert, then restore.
        // SAFETY: this is a unit test; `cargo test` serializes tests
        // inside the same file unless they explicitly parallelize,
        // and nothing here spawns a thread.
        let prior = std::env::var(REGISTRY_URL_ENV).ok();
        unsafe { std::env::remove_var(REGISTRY_URL_ENV) };
        let spec = parse_spec("@preview/basic-resume:0.2.8").unwrap();
        assert_eq!(
            tarball_url(&spec),
            "https://packages.typst.org/preview/basic-resume-0.2.8.tar.gz"
        );
        if let Some(v) = prior {
            unsafe { std::env::set_var(REGISTRY_URL_ENV, v) };
        }
    }
}
