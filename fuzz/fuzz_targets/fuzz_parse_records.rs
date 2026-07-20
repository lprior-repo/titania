//! Fuzz target: parse an arbitrary byte slice as a cargo-mutants
//! `mutants.json` payload through the pure-core
//! [`titania_core::MutantsRecords::parse_str`] API.
//!
//! The harness exercises three bounded oracles on every libFuzzer input:
//!
//! 1. **Parse-or-typed-error** — every input must surface either a
//!    successfully parsed [`MutantsRecords`] or one of the typed
//!    [`MutantsOutcomesError`] variants ([`RecordsJsonParse`],
//!    [`TooManyRecords`]). Panics, silent truncation, and any
//!    other failure shape are violations.
//! 2. **Deterministic reparse/roundtrip when successful** — when
//!    `parse_str` returns `Ok`, the value is serialised back to JSON
//!    and re-parsed; the two values must compare equal under
//!    [`PartialEq`]. Any drift is a violation.
//! 3. **Max entry caps** — a synthetic payload of
//!    `MUTANTS_RECORDS_MAX_ENTRIES + 1` minimal records is built
//!    exactly once at startup and parsed; the parser must return
//!    [`MutantsOutcomesError::TooManyRecords`]. The check is cached
//!    in a `OnceLock` so subsequent fuzz calls pay only an atomic
//!    load.
//!
//! `LLVMFuzzerTestOneInput` returns `0` on success and `-1` on any
//! oracle violation so libFuzzer records the violation as a crash.
//! The harness never uses `unwrap`, `expect`, panic, or production
//! `assert!`. The single `unsafe` block is the libFuzzer ABI slice
//! reconstruction; it is the smallest unsafe surface required by the
//! FFI contract.
//!
//! [`RecordsJsonParse`]: titania_core::MutantsOutcomesError::RecordsJsonParse
//! [`TooManyRecords`]: titania_core::MutantsOutcomesError::TooManyRecords

#![no_main]

use std::slice;
use std::sync::OnceLock;

use titania_core::{MUTANTS_RECORDS_MAX_ENTRIES, MutantsOutcomesError, MutantsRecords};

/// Path label threaded into every parser error so the lane can
/// distinguish fuzz failures from real on-disk failures.
const PATH_LABEL: &str = "<fuzz-records>";
const PATH_LABEL_RT: &str = "<fuzz-records-roundtrip>";
const PATH_LABEL_CAP: &str = "<fuzz-records-cap-oracle>";

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
    match MutantsRecords::parse_str(text, PATH_LABEL) {
        Ok(records) => roundtrip_oracle(&records),
        Err(_) => 0,
    }
}

/// Oracle 2 — reparse the serialised value and compare equality.
///
/// Returns `0` when roundtrip is lossless and `-1` on any drift.
fn roundtrip_oracle(records: &MutantsRecords) -> i32 {
    let Ok(serialised) = serde_json::to_string(records) else {
        return -1;
    };
    let Ok(reparsed) = MutantsRecords::parse_str(&serialised, PATH_LABEL_RT) else {
        return -1;
    };
    if reparsed == *records { 0 } else { -1 }
}

/// Oracle 3 — one-shot synthetic payload that exceeds
/// [`MUTANTS_RECORDS_MAX_ENTRIES`] by exactly one record.
///
/// The result is cached in a [`OnceLock<bool>`] so subsequent fuzz
/// calls pay only an atomic load. Each synthetic record carries the
/// minimum required fields (`name`, `package`, `file`); the optional
/// span/genre/replacement/function fields are left absent so the
/// payload stays small.
fn cap_oracle_satisfied() -> bool {
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        let payload = build_overflow_payload(MUTANTS_RECORDS_MAX_ENTRIES + 1);
        matches!(
            MutantsRecords::parse_str(&payload, PATH_LABEL_CAP),
            Err(MutantsOutcomesError::TooManyRecords { .. })
        )
    })
}

/// Build a minimal `mutants.json` payload carrying exactly `count`
/// record entries.
fn build_overflow_payload(count: usize) -> String {
    let mut payload = String::with_capacity(38 * count + 4);
    payload.push_str(r#"[{"name":"","package":"","file":"""#);
    for _ in 1..count {
        payload.push_str(r#"},{"name":"","package":"","file":"""#);
    }
    payload.push_str("\"}]");
    payload
}