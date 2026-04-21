//! `typst.toml` manifest parsing.
//!
//! We parse the minimum required fields for `ferrocv` to be able to
//! wire a cached package into [`crate::theme::OwnedTheme`]:
//!
//! - `package.name`
//! - `package.version`
//! - `package.entrypoint`
//!
//! Optional fields (authors, license, description, keywords, etc.)
//! are ignored. We parse via `toml::Value` rather than `serde`-derived
//! types to avoid pulling `serde-derive` into the `install` feature
//! just for three string fields.

use super::InstallError;

/// Minimal view of `typst.toml` that `ferrocv` cares about.
///
/// Built by [`parse_manifest`]; never constructed directly. The
/// `entrypoint` string is validated to be a relative path with no
/// `..` components, so Stage C's cache resolver can safely join it
/// against the package root without worrying about escapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// `package.name`.
    pub name: String,
    /// `package.version`.
    pub version: String,
    /// `package.entrypoint` — relative path inside the package.
    pub entrypoint: String,
}

/// Parse a `typst.toml` string into a [`Manifest`].
pub fn parse_manifest(toml_str: &str) -> Result<Manifest, InstallError> {
    let value: toml::Value = toml_str.parse().map_err(|e: toml::de::Error| {
        InstallError::ManifestParse {
            reason: e.to_string(),
        }
    })?;

    let package = value.get("package").ok_or_else(|| InstallError::ManifestParse {
        reason: "missing [package] table".to_owned(),
    })?;

    let name = read_required_string(package, "name")?;
    let version = read_required_string(package, "version")?;
    let entrypoint = read_required_string(package, "entrypoint")?;

    if entrypoint.is_empty() {
        return Err(InstallError::ManifestParse {
            reason: "package.entrypoint is empty".to_owned(),
        });
    }
    if entrypoint.starts_with('/') || entrypoint.starts_with('\\') {
        return Err(InstallError::ManifestParse {
            reason: format!("package.entrypoint must be a relative path: {entrypoint}"),
        });
    }
    for component in entrypoint.split(|c| c == '/' || c == '\\') {
        if component == ".." {
            return Err(InstallError::ManifestParse {
                reason: format!(
                    "package.entrypoint may not contain `..` segments: {entrypoint}"
                ),
            });
        }
    }

    Ok(Manifest {
        name,
        version,
        entrypoint,
    })
}

fn read_required_string(table: &toml::Value, key: &str) -> Result<String, InstallError> {
    let v = table.get(key).ok_or_else(|| InstallError::ManifestParse {
        reason: format!("missing package.{key}"),
    })?;
    v.as_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| InstallError::ManifestParse {
            reason: format!("package.{key} must be a string"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let src = r#"
[package]
name = "basic-resume"
version = "0.2.8"
entrypoint = "src/lib.typ"
"#;
        let m = parse_manifest(src).expect("minimal manifest parses");
        assert_eq!(m.name, "basic-resume");
        assert_eq!(m.version, "0.2.8");
        assert_eq!(m.entrypoint, "src/lib.typ");
    }

    #[test]
    fn ignores_extra_fields() {
        let src = r#"
[package]
name = "basic-resume"
version = "0.2.8"
entrypoint = "src/lib.typ"
authors = ["Some Person"]
license = "MIT"
description = "blah"
keywords = ["cv"]
exclude = [".github"]

[template]
path = "template"
entrypoint = "main.typ"
"#;
        let m = parse_manifest(src).expect("manifest with extra fields parses");
        assert_eq!(m.name, "basic-resume");
        assert_eq!(m.entrypoint, "src/lib.typ");
    }

    #[test]
    fn rejects_missing_entrypoint() {
        let src = r#"
[package]
name = "x"
version = "1"
"#;
        let err = parse_manifest(src).expect_err("missing entrypoint must fail");
        assert!(matches!(err, InstallError::ManifestParse { .. }));
    }

    #[test]
    fn rejects_absolute_entrypoint() {
        let src = r#"
[package]
name = "x"
version = "1"
entrypoint = "/etc/passwd"
"#;
        let err = parse_manifest(src).expect_err("absolute entrypoint must fail");
        match err {
            InstallError::ManifestParse { reason } => {
                assert!(reason.contains("relative"));
            }
            other => panic!("expected ManifestParse, got {other:?}"),
        }
    }

    #[test]
    fn rejects_dotdot_entrypoint() {
        let src = r#"
[package]
name = "x"
version = "1"
entrypoint = "../other/lib.typ"
"#;
        let err = parse_manifest(src).expect_err("path-traversal entrypoint must fail");
        assert!(matches!(err, InstallError::ManifestParse { .. }));
    }

    #[test]
    fn rejects_invalid_toml() {
        let err = parse_manifest("not valid toml = = =").expect_err("malformed toml must fail");
        assert!(matches!(err, InstallError::ManifestParse { .. }));
    }
}
