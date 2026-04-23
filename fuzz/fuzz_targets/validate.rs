//! Fuzz target: JSON Resume schema validation.
//!
//! Parses bytes as JSON; on parse success, drives the result through
//! `ferrocv::validate_value`. The goal is to surface panics in the
//! validator or the embedded-schema lookup path. Both Ok and Err are
//! acceptable outcomes — only panics constitute a failure.
//!
//! Part of the nightly fuzz workflow closing out issue #19.
#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let _ = ferrocv::validate_value(&value);
    }
});
