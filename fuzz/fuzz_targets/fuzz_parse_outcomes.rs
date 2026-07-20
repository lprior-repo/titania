//! Fuzz target: parse an arbitrary byte slice as a cargo-mutants
//! `outcomes.json` payload through the pure-core
//! [`titania_core::MutantsOutcomes::parse_str`] API.
//!
//! The harness exercises three bounded oracles on every libFuzzer input:
//!
//! 1. **Parse-or-typed-error** — every input must surface either a
//!    successfully parsed [`MutantsOutcomes`] or one of the typed
//!    [`MutantsOutcomesError`] variants ([`OutcomesJsonParse`],
//!    [`TooManyOutcomes`]). Panics, silent truncation, and any
//!    other failure shape are violations.
//! 2. **Deterministic reparse/roundtrip when successful** — when
//!    `parse_str` returns `Ok`, the value is serialised back to JSON
//!    and re-parsed; the two values must compare equal under
//!    [`PartialEq`]. Any drift is a violation.
//! 3. **Max entry caps** — a synthetic payload of
//!    `MUTANTS_OUTCOMES_MAX_ENTRIES + 1` minimal baseline outcomes
//!    is built exactly once at startup and parsed; the parser must
//!    return [`MutantsOutcomesError::TooManyOutcomes`]. The check is
//!    cached in a `OnceLock` so subsequent fuzz calls pay only an
//!    atomic load.
//!
//! `LLVMFuzzerTestOneInput` returns `0` on success and `-1` on any
//! oracle violation so libFuzzer records the violation as a crash.
//! The harness never uses `unwrap`, `expect`, panic, or production
//! `assert!`. The single `unsafe` block is the libFuzzer ABI slice
//! reconstruction; it is the smallest unsafe surface required by the
//! FFI contract.
//!
//! [`OutcomesJsonParse`]: titania_core::MutantsOutcomesError::OutcomesJsonParse
//! [`TooManyOutcomes`]: titania_core::MutantsOutcomesError::TooManyOutcomes

#![no_main]

use std::slice;
use std::sync::OnceLock;

use titania_core::{MUTANTS_OUTCOMES_MAX_ENTRIES, MutantsOutcomes, MutantsOutcomesError};

/// Path label threaded into every parser error so the lane can
/// distinguish fuzz failures from real on-disk failures.
const PATH_LABEL: &str = "<fuzz-outcomes>";
const PATH_LABEL_RT: &str = "<fuzz-outcomes-roundtrip>";
const PATH_LABEL_CAP: &str = "<fuzz-outcomes-cap-oracle>";

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
    match MutantsOutcomes::parse_str(text, PATH_LABEL) {
        Ok(outcomes) => roundtrip_oracle(&outcomes),
        Err(_) => 0,
    }
}

/// Oracle 2 — reparse the serialised value and compare equality.
///
/// Returns `0` when roundtrip is lossless and `-1` on any drift.
fn roundtrip_oracle(outcomes: &MutantsOutcomes) -> i32 {
    let Ok(serialised) = serde_json::to_string(outcomes) else {
        return -1;
    };
    let Ok(reparsed) = MutantsOutcomes::parse_str(&serialised, PATH_LABEL_RT) else {
        return -1;
    };
    if reparsed == *outcomes { 0 } else { -1 }
}

/// Oracle 3 — one-shot synthetic payload that exceeds
/// [`MUTANTS_OUTCOMES_MAX_ENTRIES`] by exactly one outcome.
///
/// The result is cached in a [`OnceLock<bool>`] so subsequent fuzz
/// calls pay only an atomic load. Each synthetic entry is a minimal
/// `Baseline + Success` pair so the parser never falls into the
/// `Mutant` variant — the cap oracle is about the cap path, not the
/// scenario discriminator.
fn cap_oracle_satisfied() -> bool {
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        let payload = build_overflow_payload(MUTANTS_OUTCOMES_MAX_ENTRIES + 1);
        matches!(
            MutantsOutcomes::parse_str(&payload, PATH_LABEL_CAP),
            Err(MutantsOutcomesError::TooManyOutcomes { .. })
        )
    })
}

/// Build a minimal `outcomes.json` payload carrying exactly `count`
/// baseline-success outcome entries.
fn build_overflow_payload(count: usize) -> String {
    let mut payload = String::with_capacity(48 * count + 32);
    payload.push_str(r#"{"outcomes":[{"scenario":"Baseline","summary":"Success"}"#);
    for _ in 1..count {
        payload.push_str(r#",{"scenario":"Baseline","summary":"Success"}"#);
    }
    payload.push_str("]}");
    payload
}