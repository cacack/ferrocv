//! JSON Resume schema validation.
//!
//! Validates `serde_json::Value` inputs against the embedded JSON Resume
//! v1.0.0 schema (see [`crate::JSON_RESUME_SCHEMA`]). Per
//! `CONSTITUTION.md` §6.1, no network calls happen here: the schema is
//! compiled once from the in-binary string constant and cached for the
//! lifetime of the process.
//!
//! This module is library-only. It does not know about stdin, files,
//! exit codes, or clap — those concerns live in `crate::cli`.
//!
//! # Example
//!
//! ```
//! use serde_json::json;
//!
//! let doc = json!({ "basics": { "name": "Ada Lovelace" } });
//! assert!(ferrocv::validate_value(&doc).is_ok());
//! ```
use std::fmt;
use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;

use crate::JSON_RESUME_SCHEMA;

/// A single JSON Resume schema validation failure.
///
/// `path` is a JSON Pointer (RFC 6901) to the failing instance location,
/// or the empty string for a root-level error. `message` is the
/// human-readable diagnostic produced by the underlying validator.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// JSON Pointer to the failing location, or empty for the root.
    pub path: String,
    /// Human-readable error message.
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: "<path>: <message>". The empty-path case still emits
        // the leading colon so diagnostics have a consistent shape that
        // tooling can parse without special-casing the root.
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validate `value` against the embedded JSON Resume v1.0.0 schema.
///
/// Returns `Ok(())` on success. On failure, returns `Err` with at least
/// one [`ValidationError`]; order matches the underlying validator's
/// iteration order and is not otherwise sorted or de-duplicated.
///
/// The compiled schema validator is cached in a process-wide
/// [`OnceLock`]; the first call pays the compile cost, subsequent calls
/// are cheap.
///
/// # Panics
///
/// Panics only if the embedded schema itself fails to parse or compile,
/// which would indicate a build-time bug (the schema is frozen in the
/// binary per `CONSTITUTION.md` §6.1). A schema-embedding regression
/// test in `tests/` guards this.
pub fn validate_value(value: &Value) -> Result<(), Vec<ValidationError>> {
    let validator = compiled_validator();

    // Fast path: avoid collecting errors when the document is valid.
    if validator.is_valid(value) {
        return Ok(());
    }

    let errors: Vec<ValidationError> = validator
        .iter_errors(value)
        .map(|err| ValidationError {
            path: err.instance_path().as_str().to_owned(),
            message: err.to_string(),
        })
        .collect();

    // Defensive: if the document is invalid, iter_errors must yield at
    // least one error. If this invariant is ever violated (e.g. by an
    // upstream bug), surface a synthetic diagnostic rather than
    // silently returning Ok.
    if errors.is_empty() {
        return Err(vec![ValidationError {
            path: String::new(),
            message: "document failed validation but no errors were reported".to_owned(),
        }]);
    }

    Err(errors)
}

/// Return the cached validator, compiling it on first use.
fn compiled_validator() -> &'static Validator {
    static VALIDATOR: OnceLock<Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_str(JSON_RESUME_SCHEMA)
            .expect("embedded JSON Resume schema must parse as JSON");
        // JSON Resume v1.0.0 declares `$schema: draft-04`. We pin to
        // draft 4 explicitly and enable format validation so things
        // like `format: email` / `format: uri` / `format: date` are
        // actually checked rather than treated as annotations.
        jsonschema::draft4::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("embedded JSON Resume schema must compile as a draft-04 schema")
    })
}
