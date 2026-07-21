//! Kani harnesses for pure titania-core value invariants.
//!
//! These harnesses stay behind `cfg(kani)` so normal builds keep zero Kani
//! dependency surface. They prove constructor boundaries for the new receipt
//! domain without touching filesystem-backed TargetProject behavior.

use core::str::FromStr;

// Proof obligations v15-OBL-P3-KANI-ID-KANI, v15-OBL-K1-KANI-NAME-KANI,
// and v15-OBL-K2-MUTANTS-DIFF-KANI bind these harnesses to production APIs.
use crate::{
    GateScope, KANI_HARNESS_ID_MAX_LEN, KaniHarnessId, KaniHarnessIdError, Lane, LaneDigest,
    LaneName, LaneOutcome, MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline,
    ReceiptError, ReceiptLaneExit, RecordedTargetRoot,
};

#[kani::proof]
fn lane_name_rejects_empty_string() {
    let result = LaneName::new("");
    kani::assert(matches!(result, Err(ReceiptError::EmptyLaneName)), "empty lane rejected");
}

#[kani::proof]
fn lane_name_rejects_nul_byte() {
    let result = LaneName::new("fmt\0clippy");
    kani::assert(matches!(result, Err(ReceiptError::InvalidLaneName)), "nul lane rejected");
}

#[kani::proof]
fn lane_digest_rejects_passed_greater_than_scanned() {
    let scanned: u32 = kani::any();
    let passed: u32 = kani::any();
    kani::assume(passed > scanned);
    kani::cover!(passed > scanned, "passed greater than scanned reachable");
    let lane = match LaneName::new("fmt") {
        Ok(lane) => lane,
        Err(_) => {
            kani::assert(false, "LaneName should never fail for valid 'fmt'");
            return;
        }
    };
    let result = LaneDigest::new(lane, ReceiptLaneExit::Clean, scanned, passed, 0);
    match result {
        Err(ReceiptError::PassedExceedsScanned { passed: got_passed, scanned: got_scanned }) => {
            kani::assert(got_passed == passed, "reported passed count is preserved");
            kani::assert(got_scanned == scanned, "reported scanned count is preserved");
        }
        _ => kani::assert(false, "passed count greater than scanned is rejected exactly"),
    }
}

#[kani::proof]
fn lane_digest_accepts_passed_not_greater_than_scanned() {
    let scanned: u32 = kani::any();
    let passed: u32 = kani::any();
    kani::assume(passed <= scanned);
    kani::cover!(passed == scanned, "passed equal to scanned reachable");
    kani::cover!(passed < scanned, "passed below scanned reachable");
    let lane = match LaneName::new("fmt") {
        Ok(lane) => lane,
        Err(_) => {
            kani::assert(false, "lane creation should not fail for valid input");
            return;
        }
    };
    let result = LaneDigest::new(lane, ReceiptLaneExit::Clean, scanned, passed, 0);
    match result {
        Ok(lane_digest) => {
            kani::assert(lane_digest.lane().as_str() == "fmt", "lane name is preserved");
            kani::assert(lane_digest.exit() == ReceiptLaneExit::Clean, "lane exit is preserved");
            kani::assert(lane_digest.scanned() == scanned, "scanned count is preserved");
            kani::assert(lane_digest.passed() == passed, "passed count is preserved");
            kani::assert(lane_digest.finding_count() == 0, "finding count is preserved");
        }
        Err(_) => kani::assert(false, "passed count below or equal to scanned is accepted"),
    }
}

#[kani::proof]
fn recorded_target_root_rejects_empty_string() {
    let result = RecordedTargetRoot::new("");
    kani::assert(
        matches!(result, Err(ReceiptError::TargetRootEmpty)),
        "empty target root rejected",
    );
}

#[kani::proof]
fn recorded_target_root_rejects_relative_path() {
    let result = RecordedTargetRoot::new("relative/project");
    kani::assert(
        matches!(result, Err(ReceiptError::TargetRootNonAbsolute(_))),
        "relative target root rejected",
    );
}

#[kani::proof]
fn recorded_target_root_rejects_nul_byte() {
    let result = RecordedTargetRoot::new("/tmp/project\0bad");
    kani::assert(
        matches!(result, Err(ReceiptError::TargetRootContainsNul)),
        "nul target root rejected",
    );
}

#[kani::proof]
fn recorded_target_root_accepts_absolute_path() {
    let result = RecordedTargetRoot::new("/tmp/project");
    match result {
        Ok(root) => {
            kani::assert(root.as_str() == "/tmp/project", "target root string is preserved");
        }
        Err(_) => kani::assert(false, "absolute target root is accepted"),
    }
}

const KANI_ID_CONTRACT_BOUND: usize = 32;
const KANI_DIFF_CANDIDATE_BOUND: usize = 8;
const KANI_DIFF_NOW_UNIX: u64 = 100;

// Generator support for v15-OBL-P3-KANI-ID-KANI. The mappings are
// surjective over the constructor's valid first/body ASCII byte classes.
const fn bounded_valid_first(choice: u8) -> u8 {
    let mapped = choice % 52;
    if mapped < 26 { b'a' + mapped } else { b'A' + (mapped - 26) }
}

const fn bounded_valid_body(choice: u8) -> u8 {
    let mapped = choice % 63;
    match mapped {
        0..=25 => b'a' + mapped,
        26..=51 => b'A' + (mapped - 26),
        52..=61 => b'0' + (mapped - 52),
        _ => b'_',
    }
}

const fn valid_body_byte(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte.is_ascii_digit() || byte == b'_'
}

// Obligation v15-OBL-P3-KANI-ID-KANI.
#[kani::proof]
#[kani::unwind(100)]
fn kani_kani_harness_id_bounded() {
    let symbolic_bytes: [u8; KANI_ID_CONTRACT_BOUND] = kani::any();
    let symbolic_len: usize = kani::any();
    kani::assume((1..=KANI_ID_CONTRACT_BOUND).contains(&symbolic_len));

    let mut valid_bytes = symbolic_bytes.map(bounded_valid_body);
    valid_bytes[0] = bounded_valid_first(symbolic_bytes[0]);
    let valid_candidate = match core::str::from_utf8(&valid_bytes[..symbolic_len]) {
        Ok(candidate) => candidate,
        Err(_) => {
            kani::assert(false, "ASCII generator always yields valid UTF-8");
            return;
        }
    };

    kani::cover!(
        valid_candidate.as_bytes().first().is_some_and(|byte| byte.is_ascii_lowercase()),
        "lowercase first letter is reachable"
    );
    kani::cover!(
        valid_candidate.as_bytes().first().is_some_and(|byte| byte.is_ascii_uppercase()),
        "uppercase first letter is reachable"
    );
    kani::cover!(
        valid_candidate.as_bytes().iter().skip(1).any(u8::is_ascii_digit),
        "digit body byte is reachable"
    );
    kani::cover!(
        valid_candidate.as_bytes().iter().skip(1).any(|byte| *byte == b'_'),
        "underscore body byte is reachable"
    );
    kani::cover!(symbolic_len == KANI_ID_CONTRACT_BOUND, "contract-bound length 32 is reachable");

    match KaniHarnessId::new(valid_candidate) {
        Ok(identifier) => kani::assert(
            identifier.as_str() == valid_candidate,
            "every generated valid letter-first identifier is accepted and preserved",
        ),
        Err(_) => kani::assert(false, "every generated valid identifier must be accepted"),
    }

    let invalid_first: u8 = kani::any();
    kani::assume(invalid_first.is_ascii() && !invalid_first.is_ascii_alphabetic());
    let invalid_first_bytes = [invalid_first];
    let invalid_first_candidate = match core::str::from_utf8(&invalid_first_bytes) {
        Ok(candidate) => candidate,
        Err(_) => {
            kani::assert(false, "assumed ASCII first byte is valid UTF-8");
            return;
        }
    };
    kani::cover!(invalid_first == b'0', "leading-digit rejection boundary is reachable");
    kani::cover!(invalid_first == b'_', "leading-underscore rejection is reachable");
    match KaniHarnessId::new(invalid_first_candidate) {
        Err(KaniHarnessIdError::LeadingNonLetter { byte }) => {
            kani::assert(byte == invalid_first, "leading rejection preserves the offending byte");
        }
        _ => kani::assert(false, "every ASCII non-letter first byte has the exact rejection"),
    }

    let invalid_body: u8 = kani::any();
    kani::assume(invalid_body.is_ascii() && !valid_body_byte(invalid_body));
    let invalid_body_bytes = [b'A', invalid_body];
    let invalid_body_candidate = match core::str::from_utf8(&invalid_body_bytes) {
        Ok(candidate) => candidate,
        Err(_) => {
            kani::assert(false, "assumed ASCII body byte is valid UTF-8");
            return;
        }
    };
    kani::cover!(invalid_body == b'-', "invalid-body punctuation boundary is reachable");
    match KaniHarnessId::new(invalid_body_candidate) {
        Err(KaniHarnessIdError::NotAscii { byte, offset }) => {
            kani::assert(byte == invalid_body, "body rejection preserves the offending byte");
            kani::assert(offset == 1, "body rejection reports the exact byte offset");
        }
        _ => kani::assert(false, "every disallowed ASCII body byte has the exact rejection"),
    }

    kani::assert(
        matches!(KaniHarnessId::new(""), Err(KaniHarnessIdError::Empty)),
        "fixed empty boundary has the exact rejection",
    );

    const CONTRACT_MAX: &str = concat!("AAAAAAAAAAAAAAAA", "AAAAAAAAAAAAAAAA");
    kani::assert(
        CONTRACT_MAX.len() == KANI_ID_CONTRACT_BOUND,
        "fixed contract maximum is exactly 32 bytes",
    );
    match KaniHarnessId::new(CONTRACT_MAX) {
        Ok(identifier) => kani::assert(
            identifier.as_str() == CONTRACT_MAX,
            "fixed contract-bound maximum is accepted",
        ),
        Err(_) => kani::assert(false, "fixed contract-bound maximum must be accepted"),
    }

    const OVER_GLOBAL_LIMIT: &str = concat!(
        "A",
        "AAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAA"
    );
    kani::assert(
        OVER_GLOBAL_LIMIT.len() == KANI_HARNESS_ID_MAX_LEN + 1,
        "fixed global overflow is exactly 97 bytes",
    );
    match KaniHarnessId::new(OVER_GLOBAL_LIMIT) {
        Err(KaniHarnessIdError::TooLong(length)) => kani::assert(
            length == KANI_HARNESS_ID_MAX_LEN + 1,
            "global overflow rejection preserves the exact length",
        ),
        _ => kani::assert(false, "fixed over-global-limit input has the exact rejection"),
    }
}

// Deterministic verification-only mapping for v15-OBL-K1-KANI-NAME-KANI.
// It uses the public file stem and does not model the private shell normalizer.
fn bounded_kani_lane_name(lane: Lane) -> String {
    let stem = lane.file_stem();
    let mut name = String::with_capacity("kani_".len() + stem.len());
    name.push_str("kani_");
    stem.bytes().for_each(|byte| {
        let mapped = if byte == b'-' { b'_' } else { byte.to_ascii_lowercase() };
        name.push(char::from(mapped));
    });
    name
}

// Obligation v15-OBL-K1-KANI-NAME-KANI.
#[kani::proof]
#[kani::unwind(32)]
fn kani_kani_lane_name_roundtrip() {
    GateScope::Full.lanes().iter().for_each(|lane| {
        let harness_name = bounded_kani_lane_name(*lane);
        kani::assert(
            harness_name.len() <= KANI_ID_CONTRACT_BOUND,
            "every full-scope lane maps to a bounded Kani-style name",
        );
        kani::assert(
            harness_name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
            "mapped Kani-style names contain only lowercase ASCII, digits, and underscores",
        );
        match KaniHarnessId::new(&harness_name) {
            Ok(identifier) => kani::assert(
                identifier.as_str() == harness_name,
                "the production constructor accepts and preserves the mapped lane name",
            ),
            Err(_) => kani::assert(false, "every full-scope mapped lane name must be accepted"),
        }
        kani::assert(
            Lane::from_str(lane.name()) == Ok(*lane),
            "the production lane name parses back to the same lane",
        );
        kani::cover!(*lane == Lane::Kani, "the Full scope reaches Lane::Kani");
        kani::cover!(*lane == Lane::Mutants, "the Full scope reaches Lane::Mutants");
    });
}

// Helper functions used by `kani_mutants_baseline_diff_zero_neg`. Pure
// data-flow: they decide whether a slot's entry is "selected" and
// "not expired" so the production `MutantsBaseline::contains` returns
// `true` iff the row is active. Extracted to keep the harness body
// below the 60-line review ceiling.
const fn diff_entry_expiry(selected: bool, expired: bool) -> Option<u64> {
    if !selected {
        None
    } else if expired {
        Some(KANI_DIFF_NOW_UNIX - 1)
    } else {
        Some(KANI_DIFF_NOW_UNIX)
    }
}

fn diff_entry_rule(selected: bool, expired: bool) -> String {
    if !selected || expired {
        "mutant-accept/proof/bounded/99".to_owned()
    } else {
        "mutant-accept/proof/bounded/100".to_owned()
    }
}

const DIFF_PROOF_REASON: &str = "bounded Kani diff proof";

// Validate one fixed-shape candidate `MutantId::new` call. Returns the
// candidate on `Ok`; on `Err` it emits `kani::assert(false)` so Kani
// fails any path that produces an error, then synthesises a known-valid
// candidate via a recursive call (line=1 always succeeds) so the type
// checker is satisfied. The recursive call is total: every iteration of
// `MutantId::new("p", "a", line, 1, MutantOperator::AndOr)` for line in
// 1..=8 succeeds because the production constructor accepts those exact
// arguments.
fn build_kani_diff_candidate(line: u32) -> MutantId {
    match MutantId::new("p", "a", line, 1, MutantOperator::AndOr) {
        Ok(candidate) => candidate,
        Err(_) => {
            kani::assert(false, "fixed-shape candidate MutantId must validate");
            // The branch above is unreachable in verification; Kani marks
            // any path reaching it as failed. The compiler still needs a
            // `MutantId`, so we call back into the helper with the same
            // known-good shape (line=1) used elsewhere in this harness.
            build_kani_diff_candidate(1)
        }
    }
}

// Obligation v15-OBL-K2-MUTANTS-DIFF-KANI.
//
// Bound strategy: 8 fixed-shape candidates + 24 symbolic bits across
// three `kani::any` bool arrays. To keep CBMC tractable we always
// materialise exactly 8 `MutantBaselineEntry` items in a fixed-length
// `Vec` and pass all 8 candidates to `MutantsBaseline::diff`. The
// "selection" and "expiry" booleans gate which entries actually cover
// each candidate: unselected entries carry `expires_on_unix = None`,
// expired entries carry expiry < `KANI_DIFF_NOW_UNIX`, so
// `MutantsBaseline::contains` returns `false` for them, and the
// survivor-superset property is exercised symbolically without
// triggering the symbolic `Vec` length explosion that gated pushes
// would produce.
#[kani::proof]
#[kani::unwind(32)]
fn kani_mutants_baseline_diff_zero_neg() {
    let candidates: [MutantId; KANI_DIFF_CANDIDATE_BOUND] = [
        build_kani_diff_candidate(1),
        build_kani_diff_candidate(2),
        build_kani_diff_candidate(3),
        build_kani_diff_candidate(4),
        build_kani_diff_candidate(5),
        build_kani_diff_candidate(6),
        build_kani_diff_candidate(7),
        build_kani_diff_candidate(8),
    ];
    let survivor_selected: [bool; KANI_DIFF_CANDIDATE_BOUND] = kani::any();
    let baseline_selected: [bool; KANI_DIFF_CANDIDATE_BOUND] = kani::any();
    let baseline_expired: [bool; KANI_DIFF_CANDIDATE_BOUND] = kani::any();

    let baseline_entries: [MutantBaselineEntry; KANI_DIFF_CANDIDATE_BOUND] = [
        MutantBaselineEntry {
            mutation_id: candidates[0].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[0], baseline_expired[0]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[0], baseline_expired[0]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[1].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[1], baseline_expired[1]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[1], baseline_expired[1]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[2].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[2], baseline_expired[2]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[2], baseline_expired[2]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[3].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[3], baseline_expired[3]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[3], baseline_expired[3]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[4].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[4], baseline_expired[4]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[4], baseline_expired[4]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[5].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[5], baseline_expired[5]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[5], baseline_expired[5]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[6].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[6], baseline_expired[6]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[6], baseline_expired[6]),
        },
        MutantBaselineEntry {
            mutation_id: candidates[7].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[7], baseline_expired[7]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[7], baseline_expired[7]),
        },
    ];

    let baseline = MutantsBaseline::from_bypasses(baseline_entries.to_vec());
    // Production `diff` called with the full fixed-length candidate slice
    // so the returned `Vec<&MutantId>` length is bounded but symbolically
    // filtered; the Vec's heap length state stays under 2^8 = 256 paths
    // instead of the 2^24 path explosion produced by gated pushes.
    let diff = baseline.diff(&candidates, KANI_DIFF_NOW_UNIX);

    // Survivors ⊆ diff ∪ baseline.contains: every survivor not covered by
    // an active baseline entry must appear in the diff, and every covered
    // survivor must be excluded from it. The `survivor_selected` bits gate
    // which candidates count as "real" survivors for the assertion so the
    // harness still distinguishes "candidate never treated as a survivor"
    // from "candidate should be in diff".
    (0..KANI_DIFF_CANDIDATE_BOUND).for_each(|index| {
        let candidate = &candidates[index];
        let in_diff = diff.iter().any(|entry| *entry == candidate);
        let covered = baseline.contains(candidate, KANI_DIFF_NOW_UNIX);

        if survivor_selected[index] {
            if covered {
                kani::assert(!in_diff, "covered survivor is excluded from diff");
            } else {
                kani::assert(in_diff, "every uncovered survivor is retained in diff");
            }
        }

        let has_expired_matching_entry = baseline_entries.iter().any(|entry| {
            entry.mutation_id == *candidate
                && entry.expires_on_unix.is_some_and(|expiry| expiry < KANI_DIFF_NOW_UNIX)
        });
        if has_expired_matching_entry && survivor_selected[index] {
            kani::assert(in_diff, "an expired matching entry does not suppress its survivor");
        }
    });

    // `diff` never returns an identifier outside the survivor universe.
    // With `survivors == &candidates` this is the strongest invariant the
    // production `diff` can offer: every emitted reference points into the
    // input slice.
    diff.iter().for_each(|entry| {
        kani::assert(
            candidates.iter().any(|survivor| survivor == *entry),
            "diff never returns an identifier outside the survivor slice",
        );
    });

    let any_covered_selected = (0..KANI_DIFF_CANDIDATE_BOUND)
        .any(|i| survivor_selected[i] && baseline.contains(&candidates[i], KANI_DIFF_NOW_UNIX));
    let any_uncovered_selected = (0..KANI_DIFF_CANDIDATE_BOUND)
        .any(|i| survivor_selected[i] && !baseline.contains(&candidates[i], KANI_DIFF_NOW_UNIX));
    let any_expired_entry = baseline_entries
        .iter()
        .any(|entry| entry.expires_on_unix.is_some_and(|expiry| expiry < KANI_DIFF_NOW_UNIX));
    let all_baseline_active = baseline_selected.iter().all(|selected| *selected)
        && baseline_expired.iter().all(|expired| !*expired);
    let all_survivors_selected = survivor_selected.iter().all(|selected| *selected);
    let no_survivor_selected = survivor_selected.iter().all(|selected| !*selected);

    kani::cover!(no_survivor_selected, "no-survivor selection is reachable");
    kani::cover!(all_survivors_selected, "all eight survivors selected is reachable");
    kani::cover!(
        baseline_selected.iter().all(|selected| !*selected),
        "no baseline selection is reachable"
    );
    kani::cover!(
        all_baseline_active,
        "all eight baseline entries active simultaneously is reachable"
    );
    kani::cover!(
        any_covered_selected && any_uncovered_selected,
        "covered and uncovered selected survivors coexist"
    );
    kani::cover!(any_expired_entry, "expired baseline entry is reachable");
}

/// Wire-mirror invariant for [`LaneOutcome`]: any bounded symbolic
/// byte sequence either deserialises to a value whose serialise/reparse
/// round-trip is the identity, or surfaces an `Err` (the deserialiser
/// maps every typed `OutcomeError` variant through
/// `serde::de::Error::custom`). No panics, no process exit. Buffer is
/// 8 bytes; `unwind(64)` covers serde_json's internal state-machine.
#[kani::proof]
#[kani::unwind(64)]
fn outcome_wire_invariants() {
    let bytes: [u8; 8] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= bytes.len());
    let Ok(text) = core::str::from_utf8(&bytes[..len]) else { return };
    let Ok(value) = serde_json::from_str::<LaneOutcome>(text) else { return };
    let Ok(serialised) = serde_json::to_string(&value) else {
        kani::assert(false, "serialise must succeed");
        return;
    };
    let Ok(reparsed) = serde_json::from_str::<LaneOutcome>(&serialised) else {
        kani::assert(false, "reparse must succeed");
        return;
    };
    kani::assert(reparsed == value, "roundtrip preserves LaneOutcome");
}
