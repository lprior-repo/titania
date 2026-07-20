//! Fuzz target: parse an arbitrary byte slice as a cargo-kani
//! `kani-list.json` payload through the pure-core
//! [`titania_core::KaniInventory::parse_str`] API.
//!
//! The harness exercises three bounded oracles on every libFuzzer input:
//!
//! 1. **Parse-or-typed-error** — every input must surface either a
//!    successfully parsed [`KaniInventory`] or one of the typed
//!    [`KaniInventoryError`] variants ([`JsonParse`],
//!    [`TooManyHarnesses`]). Panics, silent truncation, and any
//!    other failure shape are violations.
//! 2. **Deterministic reparse/roundtrip when successful** — when
//!    `parse_str` returns `Ok`, the value is serialised back to JSON
//!    and re-parsed; the two values must compare equal under
//!    [`PartialEq`]. Any drift is a violation.
//! 3. **Max entry caps** — a synthetic payload of
//!    `KANI_INVENTORY_MAX_HARNESSES + 1` minimal harnesses is built
//!    exactly once at startup and parsed; the parser must return
//!    [`KaniInventoryError::TooManyHarnesses`]. The check is cached
//!    in a `OnceLock` so subsequent fuzz calls pay only an atomic load.
//!
//! `LLVMFuzzerTestOneInput` returns `0` on success and `-1` on any
//! oracle violation so libFuzzer records the violation as a crash.
//! The harness never uses `unwrap`, `expect`, panic, or production
//! `assert!`. The single `unsafe` block is the libFuzzer ABI slice
//! reconstruction; it is the smallest unsafe surface required by the
//! FFI contract.
//!
//! [`JsonParse`]: titania_core::KaniInventoryError::JsonParse
//! [`TooManyHarnesses`]: titania_core::KaniInventoryError::TooManyHarnesses

#![no_main]

use std::slice;
use std::sync::OnceLock;

use titania_core::{KANI_INVENTORY_MAX_HARNESSES, KaniInventory, KaniInventoryError};

/// Path label threaded into every parser error so the lane can
/// distinguish fuzz failures from real on-disk failures.
const PATH_LABEL: &str = "<fuzz-inventory>";
const PATH_LABEL_RT: &str = "<fuzz-inventory-roundtrip>";
const PATH_LABEL_CAP: &str = "<fuzz-inventory-cap-oracle>";

/// libFuzzer entry point.
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInput(data: *const u8, size: usize) -> i32 {
    if !cap_oracle_satisfied() {
        return -1;
    }

    // SAFETY: libFuzzer guarantees that `data` points to `size`
    // readable bytes that remain live for the duration of this call.
    // This is the only `unsafe` block in the harness and exists solely
    // to honour the libFuzzer C ABI.
    let bytes = unsafe { slice::from_raw_parts(data, size) };

    // The parsers consume `&str`, so non-UTF8 fuzz data is rejected
    // up-front. A non-UTF8 slice is not an oracle violation: the
    // parser contract is `&str → Result`, not `&[u8] → Result`.
    let Ok(text) = core::str::from_utf8(bytes) else {
        return 0;
    };

    // Oracle 1: parse-or-typed-error.
    // Oracle 2: deterministic reparse/roundtrip on success.
    match KaniInventory::parse_str(text, PATH_LABEL) {
        Ok(inventory) => roundtrip_oracle(&inventory),
        Err(_) => 0,
    }
}

/// Oracle 2 — reparse the serialised value and compare equality.
///
/// Returns `0` when roundtrip is lossless and `-1` on any drift.
fn roundtrip_oracle(inventory: &KaniInventory) -> i32 {
    let Ok(serialised) = serde_json::to_string(inventory) else {
        return -1;
    };
    let Ok(reparsed) = KaniInventory::parse_str(&serialised, PATH_LABEL_RT) else {
        return -1;
    };
    if reparsed == *inventory { 0 } else { -1 }
}

/// Oracle 3 — one-shot synthetic payload that exceeds
/// [`KANI_INVENTORY_MAX_HARNESSES`] by exactly one harness.
///
/// The result is cached in a [`OnceLock<bool>`] so subsequent fuzz
/// calls pay only an atomic load. The construction itself is O(n)
/// in `push_str` (amortised), and parsing 1M+1 entries is bounded
/// by the parser's per-entry allocation profile.
fn cap_oracle_satisfied() -> bool {
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        let payload = build_overflow_payload(KANI_INVENTORY_MAX_HARNESSES + 1);
        matches!(
            KaniInventory::parse_str(&payload, PATH_LABEL_CAP),
            Err(KaniInventoryError::TooManyHarnesses { .. })
        )
    })
}

/// Build a minimal `kani-list.json` payload carrying exactly `count`
/// harnesses under a single empty-key source file.
///
/// Each harness name is the literal byte `b` so the canonicalisation
/// path stays inside the closed-set ASCII branch — the cap oracle
/// must not be coupled to arbitrary UTF-8 surface area.
fn build_overflow_payload(count: usize) -> String {
    let mut payload = String::with_capacity(4 * count + 64);
    payload.push_str(r#"{"standard-harnesses":{"a":["b""#);
    for _ in 1..count {
        payload.push_str(r#","b""#);
    }
    payload.push_str(r#"]}}"#);
    payload
}