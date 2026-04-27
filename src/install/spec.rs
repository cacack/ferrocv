//! Parse `@preview/<name>:<version>` package specs.
//!
//! v1 accepts exactly one namespace (`@preview/`). Other Typst
//! namespaces (`@local/`, `@x-...`) are rejected with a clear error
//! pointing at the v1-scope decision; adding them is a follow-up, not
//! a silent extension.

use super::InstallError;

/// A parsed Typst Universe package spec.
///
/// Constructed by [`parse_spec`]; never constructed directly. The
/// `namespace` field always equals `"preview"` in v1 — the field
/// exists so Stage C's cache resolver and the installer URL builder
/// can share one source of truth if/when we expand past `@preview/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    /// Typst namespace. Always `"preview"` in v1.
    pub namespace: String,
    /// Package name, the `<name>` in `@<namespace>/<name>:<version>`.
    pub name: String,
    /// SemVer string, the `<version>` in
    /// `@<namespace>/<name>:<version>`. Not parsed further — we pass
    /// it through verbatim to the registry URL and to the cache path.
    pub version: String,
}

/// Parse `@preview/<name>:<version>` into a [`PackageSpec`].
///
/// Only `@preview/` is accepted in v1. The name and version are
/// validated syntactically (non-empty, no slashes, no whitespace, no
/// path-traversal characters) so they are safe to interpolate into
/// URLs and filesystem paths. We intentionally do NOT parse the
/// version as SemVer — the registry accepts whatever version strings
/// it accepts, and locking that choice inside the CLI would diverge
/// from the registry without buying anything.
pub fn parse_spec(raw: &str) -> Result<PackageSpec, InstallError> {
    // Must start with @preview/; anything else is rejected up front.
    let Some(after_preview) = raw.strip_prefix("@preview/") else {
        let reason = if raw.starts_with('@') {
            // A different namespace — point the user at the v1 scope.
            "only the @preview/ namespace is supported in v1".to_owned()
        } else {
            "spec must start with @preview/".to_owned()
        };
        return Err(InstallError::InvalidSpec {
            raw: raw.to_owned(),
            reason,
        });
    };

    // Split name from version at the LAST `:` so a version like
    // `1.2.3-beta.4` with internal punctuation round-trips cleanly.
    // Every legal SemVer version can contain `.` and `-` but not `:`,
    // so the last `:` is unambiguous.
    let Some((name, version)) = after_preview.rsplit_once(':') else {
        return Err(InstallError::InvalidSpec {
            raw: raw.to_owned(),
            reason: "missing `:<version>` after package name".to_owned(),
        });
    };

    if name.is_empty() {
        return Err(InstallError::InvalidSpec {
            raw: raw.to_owned(),
            reason: "package name is empty".to_owned(),
        });
    }
    if version.is_empty() {
        return Err(InstallError::InvalidSpec {
            raw: raw.to_owned(),
            reason: "version string is empty".to_owned(),
        });
    }

    // Defensive validation — keeps malicious or malformed inputs from
    // reaching the URL builder or the cache path joiner.
    validate_component("name", name, raw)?;
    validate_component("version", version, raw)?;

    Ok(PackageSpec {
        namespace: "preview".to_owned(),
        name: name.to_owned(),
        version: version.to_owned(),
    })
}

/// Validate that a spec component (name or version) is safe to use in
/// a URL path segment and a filesystem component.
///
/// Rejects: empty strings, whitespace, path separators (`/`, `\`),
/// path-traversal sequences (`..`), and any control character. This
/// is intentionally stricter than the registry itself; we'd rather
/// reject a weird-but-legal name than chance shell-injection or
/// path-traversal in the cache path.
fn validate_component(what: &'static str, value: &str, raw: &str) -> Result<(), InstallError> {
    if value == "." || value == ".." {
        return Err(InstallError::InvalidSpec {
            raw: raw.to_owned(),
            reason: format!("{what} must not be `.` or `..`"),
        });
    }
    for ch in value.chars() {
        if ch.is_whitespace()
            || ch.is_control()
            || ch == '/'
            || ch == '\\'
            || ch == ':'
            || ch == '?'
            || ch == '#'
        {
            return Err(InstallError::InvalidSpec {
                raw: raw.to_owned(),
                reason: format!("{what} contains illegal character `{}`", ch.escape_debug()),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_spec() {
        let spec = parse_spec("@preview/basic-resume:0.2.8").expect("canonical spec parses");
        assert_eq!(spec.namespace, "preview");
        assert_eq!(spec.name, "basic-resume");
        assert_eq!(spec.version, "0.2.8");
    }

    #[test]
    fn parses_prerelease_version() {
        let spec =
            parse_spec("@preview/cetz:0.3.1-beta.2").expect("prerelease versions pass through");
        assert_eq!(spec.version, "0.3.1-beta.2");
    }

    #[test]
    fn rejects_missing_prefix() {
        let err = parse_spec("basic-resume:0.2.8").expect_err("bare name must fail");
        match err {
            InstallError::InvalidSpec { raw, reason } => {
                assert_eq!(raw, "basic-resume:0.2.8");
                assert!(
                    reason.contains("@preview/"),
                    "error must hint at @preview/ prefix: {reason}"
                );
            }
            other => panic!("expected InvalidSpec, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_preview_namespace() {
        let err = parse_spec("@local/mine:1.0").expect_err("@local/ is not supported in v1");
        match err {
            InstallError::InvalidSpec { raw: _, reason } => {
                assert!(
                    reason.contains("@preview/"),
                    "error must point at @preview/ as the supported namespace: {reason}"
                );
            }
            other => panic!("expected InvalidSpec, got {other:?}"),
        }
    }

    #[test]
    fn rejects_missing_version() {
        let err = parse_spec("@preview/basic-resume").expect_err("missing version must fail");
        assert!(matches!(err, InstallError::InvalidSpec { .. }));
    }

    #[test]
    fn rejects_empty_name() {
        let err = parse_spec("@preview/:1.0").expect_err("empty name must fail");
        assert!(matches!(err, InstallError::InvalidSpec { .. }));
    }

    #[test]
    fn rejects_empty_version() {
        let err = parse_spec("@preview/foo:").expect_err("empty version must fail");
        assert!(matches!(err, InstallError::InvalidSpec { .. }));
    }

    #[test]
    fn rejects_path_traversal_in_name() {
        let err = parse_spec("@preview/../evil:1.0").expect_err("path separator rejected");
        assert!(matches!(err, InstallError::InvalidSpec { .. }));
    }

    #[test]
    fn rejects_whitespace_in_version() {
        let err = parse_spec("@preview/foo:1.0 ").expect_err("whitespace rejected");
        assert!(matches!(err, InstallError::InvalidSpec { .. }));
    }
}
