//! Shared test-only utilities. Compiled only under `#[cfg(test)]`, so
//! it never ships in any release artifact.
//!
//! Currently exports a single mutex used to serialize env-var
//! mutations across unit tests in different modules. ferrocv has
//! multiple test sites that mutate `FERROCV_CACHE_DIR`
//! ([`crate::install::cache::tests`], [`crate::package_cache::tests`],
//! and [`crate::theme::tests`]); without a process-wide lock,
//! `cargo test`'s parallel threads would race on the env var and
//! produce intermittent failures. One static lock here is the smallest
//! solution that keeps all three test modules honest.

use std::sync::Mutex;

/// Process-wide serialization lock for env-var-mutating tests.
///
/// Acquire before any `unsafe { std::env::set_var(...) }` call in a
/// test. The lock is poison-tolerant: a panic inside one test releases
/// the lock and the next test recovers via `unwrap_or_else(|p|
/// p.into_inner())`.
pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());
