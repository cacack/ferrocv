//! Fuzz target: raw JSON parsing.
//!
//! Feeds arbitrary bytes to `serde_json::from_slice::<Value>`. The goal
//! is to surface panics or UB in the parser path, not to assert any
//! semantic property — parse failures are expected on random input.
//!
//! Part of the nightly fuzz workflow closing out issue #19.
#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Value>(data);
});
