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

/// Default registry URL prefix. Override via [`fetch_tarball_from`]
/// in tests to point at a local fixture server.
pub const DEFAULT_REGISTRY: &str = "https://packages.typst.org/preview";

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

/// Construct the canonical tarball URL for a spec against
/// [`DEFAULT_REGISTRY`].
pub fn tarball_url(spec: &PackageSpec) -> String {
    format!(
        "{DEFAULT_REGISTRY}/{name}-{version}.tar.gz",
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
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
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
    reader.read_to_end(&mut buf).map_err(|source| InstallError::Io {
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
        let spec = parse_spec("@preview/basic-resume:0.2.8").unwrap();
        assert_eq!(
            tarball_url(&spec),
            "https://packages.typst.org/preview/basic-resume-0.2.8.tar.gz"
        );
    }
}
