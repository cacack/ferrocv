//! `ferrocv` renders JSON Resume documents to PDF, HTML, and plain text
//! via embedded Typst. See `CONSTITUTION.md` for design principles.

pub mod cli;
pub mod validate;

pub use validate::{ValidationError, validate_value};

/// JSON Resume v1.0.0 schema, embedded at compile time.
///
/// Per `CONSTITUTION.md` section 6.1, `ferrocv` makes no network calls at
/// render or validate time. The schema is vendored under `assets/schema/`
/// and frozen per release; bumping it is an intentional release action.
pub const JSON_RESUME_SCHEMA: &str = include_str!("../assets/schema/jsonresume-v1.0.0.json");

/// Version of the embedded JSON Resume schema (see [`JSON_RESUME_SCHEMA`]).
///
/// Pinned at compile time alongside the schema bytes. See
/// `CONSTITUTION.md` section 6.1 for the no-network commitment that this
/// embedding upholds.
pub const JSON_RESUME_SCHEMA_VERSION: &str = "1.0.0";
