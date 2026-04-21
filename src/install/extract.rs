//! Tarball extraction via `flate2` + `tar`.
//!
//! Typst Universe tarballs are flat (files at the archive root, no
//! `<name>-<version>/` wrapping directory), so `tar::Archive::unpack`
//! writes directly into `dest`.
//!
//! Path-traversal safety: the `tar` crate's `unpack()` sanitizes
//! entry paths by default (it refuses absolute paths and `..`
//! components that escape `dest`). We add a belt-and-suspenders pass
//! that walks every entry and rejects anything suspect before calling
//! `unpack()` — cheap to do and catches any future regression in the
//! crate's sanitization.

use std::io::Cursor;
use std::path::{Component, Path};

use flate2::read::GzDecoder;
use tar::Archive;

use super::InstallError;

/// Extract a gzipped tar into `dest`.
///
/// `dest` is expected to be an empty directory created by the caller
/// (typically a `tempfile::TempDir`).
pub fn extract_tarball(bytes: &[u8], dest: &Path) -> Result<(), InstallError> {
    // First pass: walk entries to reject malicious paths before we
    // write anything to disk.
    {
        let gz = GzDecoder::new(Cursor::new(bytes));
        let mut archive = Archive::new(gz);
        let entries = archive.entries().map_err(|source| InstallError::Extract {
            context: "read tar entries".to_owned(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| InstallError::Extract {
                context: "read tar entry".to_owned(),
                source,
            })?;
            let path = entry.path().map_err(|source| InstallError::Extract {
                context: "decode tar entry path".to_owned(),
                source,
            })?;
            reject_unsafe_path(&path)?;
        }
    }

    // Second pass: actually extract.
    let gz = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(gz);
    archive.unpack(dest).map_err(|source| InstallError::Extract {
        context: format!("unpack tarball into {}", dest.display()),
        source,
    })?;
    Ok(())
}

/// Reject any tar entry path that is absolute or contains `..`
/// components.
fn reject_unsafe_path(path: &Path) -> Result<(), InstallError> {
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => {
                return Err(InstallError::Extract {
                    context: format!(
                        "tar entry uses `..` path traversal: {}",
                        path.display(),
                    ),
                    source: std::io::Error::other("unsafe tar entry path"),
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(InstallError::Extract {
                    context: format!(
                        "tar entry uses absolute path: {}",
                        path.display(),
                    ),
                    source: std::io::Error::other("absolute tar entry path"),
                });
            }
        }
    }
    Ok(())
}
