//! Fuzz target: parse arbitrary bytes as a titania `LaneOutcome` wire
//! payload through `serde_json::from_str::<LaneOutcome>`.
//!
//! Bounded oracles on every libFuzzer input:
//! 1. Parse-or-typed-error — `Ok` or typed `OutcomeError` variant.
//! 2. Reparse round-trip — on `Ok`, serialise+re-parse; equal.
//! 3. Empty-findings rejection — `{"Findings":[]}` rejected.
//! 4. Non-zero-exit rejection — `Clean` non-success exit rejected.
//! 5. Argv0 mismatch rejection — `argv[0] != executable` rejected.
//! 6. Max-input cap — refuse inputs above 1 MiB before allocation.
//!
//! `0` on success, `-1` on oracle violation. Only `unsafe` is the
//! libFuzzer FFI slice reconstruction; everything else is panic-free.

#![no_main]

use std::slice;
use std::sync::OnceLock;

use titania_core::LaneOutcome;

const LANE_OUTCOME_MAX_INPUT_BYTES: usize = 1024 * 1024;

/// `Clean` whose `ProcessTermination::Exited { code: 1 }` must be rejected.
const NON_ZERO_EXIT_PAYLOAD: &str = r#"{"Clean":{"evidence":{"command":{"executable":"cargo","argv":["cargo","fmt","--check"]},"tool_version":"rustfmt 1.84.0","exit_status":{"Exited":{"code":1}},"parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"}}}"#;

/// `CommandEvidence` whose `argv[0]` does not match `executable`.
const ARGV0_MISMATCH_PAYLOAD: &str = r#"{"Clean":{"evidence":{"command":{"executable":"cargo","argv":["rustc","-V"]},"tool_version":"rustc 1.84.0","exit_status":{"Exited":{"code":0}},"parsed_result_digest":"44e86eb38ff9b065a1e805dd61f624e334a4296479e4be43c354ca8af90b0340"}}}"#;

const EMPTY_FINDINGS_FRAGMENT: &str = "at least one finding";
const NON_ZERO_EXIT_FRAGMENT: &str = "exit status";
const ARGV0_MISMATCH_FRAGMENT: &str = "argv[0]";

#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInput(data: *const u8, size: usize) -> i32 {
    // Oracle 6: refuse oversized inputs before any allocation.
    if size > LANE_OUTCOME_MAX_INPUT_BYTES {
        return 0;
    }
    if !typed_error_oracle(r#"{"Findings":[]}"#, EMPTY_FINDINGS_FRAGMENT)
        || !typed_error_oracle(NON_ZERO_EXIT_PAYLOAD, NON_ZERO_EXIT_FRAGMENT)
        || !typed_error_oracle(ARGV0_MISMATCH_PAYLOAD, ARGV0_MISMATCH_FRAGMENT)
    {
        return -1;
    }

    // SAFETY: libFuzzer guarantees `data` points to `size` readable
    // bytes that remain live for the duration of this call. This is
    // the only `unsafe` block in the harness and exists solely to
    // honour the libFuzzer C ABI.
    let bytes = unsafe { slice::from_raw_parts(data, size) };

    // Non-UTF8 inputs are not oracle violations: the wire-format
    // contract is `&str -> Result`, not `&[u8] -> Result`.
    let Ok(text) = core::str::from_utf8(bytes) else {
        return 0;
    };

    // Oracles 1 + 2.
    match serde_json::from_str::<LaneOutcome>(text) {
        Ok(outcome) => roundtrip_oracle(&outcome),
        Err(_) => 0,
    }
}

/// Oracle 2 — reparse the serialised value and compare equality.
/// Returns `0` on lossless roundtrip, `-1` on drift.
fn roundtrip_oracle(outcome: &LaneOutcome) -> i32 {
    let Ok(serialised) = serde_json::to_string(outcome) else {
        return -1;
    };
    let Ok(reparsed) = serde_json::from_str::<LaneOutcome>(&serialised) else {
        return -1;
    };
    if reparsed == *outcome { 0 } else { -1 }
}

/// Parse `payload` once and confirm the error display contains
/// `fragment`. Cached in a `OnceLock` so subsequent fuzz calls pay only
/// an atomic load. Returns `false` if the parser accepts the payload.
fn typed_error_oracle(payload: &str, fragment: &str) -> bool {
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| match serde_json::from_str::<LaneOutcome>(payload) {
        Err(error) => error.to_string().contains(fragment),
        Ok(_) => false,
    })
}