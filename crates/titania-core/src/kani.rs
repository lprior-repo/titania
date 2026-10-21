use core::str::FromStr;

use crate::{
    CommandEvidence, Digest, Finding, KANI_HARNESS_ID_MAX_LEN, KaniHarnessId, KaniHarnessIdError,
    Lane, LaneDigest, LaneEvidence, LaneFailure, LaneName, LaneOutcome, Location,
    MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline, ProcessTermination,
    ReceiptError, ReceiptLaneExit, RecordedTargetRoot, RepairHint, RuleId, SkipReason,
    WorkspacePath,
};

#[kani::proof]
#[kani::unwind(32)]
fn lane_name_rejects_empty_string() {
    let result = LaneName::new("");
    kani::assert(matches!(result, Err(ReceiptError::EmptyLaneName)), "empty lane rejected");
}

#[kani::proof]
#[kani::unwind(32)]
fn lane_name_rejects_nul_byte() {
    let result = LaneName::new("fmt\0clippy");
    kani::assert(matches!(result, Err(ReceiptError::InvalidLaneName)), "nul lane rejected");
}

#[kani::proof]
#[kani::unwind(32)]
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
#[kani::unwind(32)]
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
#[kani::unwind(32)]
fn recorded_target_root_rejects_empty_string() {
    let result = RecordedTargetRoot::new("");
    kani::assert(
        matches!(result, Err(ReceiptError::TargetRootEmpty)),
        "empty target root rejected",
    );
}

#[kani::proof]
#[kani::unwind(32)]
fn recorded_target_root_rejects_relative_path() {
    let result = RecordedTargetRoot::new("relative/project");
    kani::assert(
        matches!(result, Err(ReceiptError::TargetRootNonAbsolute(_))),
        "relative target root rejected",
    );
}

#[kani::proof]
#[kani::unwind(32)]
fn recorded_target_root_rejects_nul_byte() {
    let result = RecordedTargetRoot::new("/tmp/project\0bad");
    kani::assert(
        matches!(result, Err(ReceiptError::TargetRootContainsNul)),
        "nul target root rejected",
    );
}

#[kani::proof]
#[kani::unwind(32)]
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

#[kani::proof]
#[kani::unwind(34)]
fn kani_harness_id_valid_bounded() {
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
}
#[kani::proof]
#[kani::unwind(32)]
fn kani_harness_id_invalid_first() {
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
}
#[kani::proof]
#[kani::unwind(100)]
fn kani_harness_id_invalid_body_and_fixed() {
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

const ALL_LANES: [Lane; 12] = [
    Lane::Fmt,
    Lane::Compile,
    Lane::Clippy,
    Lane::AstGrep,
    Lane::Dylint,
    Lane::PanicScan,
    Lane::PolicyScan,
    Lane::Test,
    Lane::Deny,
    Lane::Build,
    Lane::Kani,
    Lane::Mutants,
];

#[kani::proof]
#[kani::unwind(32)]
fn kani_kani_lane_name_roundtrip() {
    kani::assert(!ALL_LANES.is_empty(), "the lane catalog is non-empty");
    ALL_LANES.iter().for_each(|lane| {
        let harness_name = bounded_kani_lane_name(*lane);
        kani::assert(
            harness_name.len() <= KANI_ID_CONTRACT_BOUND,
            "every lane maps to a bounded Kani-style name",
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
            Err(_) => kani::assert(false, "every mapped lane name must be accepted"),
        }
        kani::assert(
            Lane::from_str(lane.name()) == Ok(*lane),
            "the production lane name parses back to the same lane",
        );
        kani::cover!(*lane == Lane::Kani, "the lane catalog reaches Lane::Kani");
        kani::cover!(*lane == Lane::Mutants, "the lane catalog reaches Lane::Mutants");
    });
}

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

fn build_kani_diff_candidate(line: u32) -> MutantId {
    let mut candidate_line = line;
    loop {
        match MutantId::new("p", "a", candidate_line, 1, MutantOperator::AndOr) {
            Ok(candidate) => return candidate,
            Err(_) => {
                kani::assert(false, "fixed-shape candidate MutantId must validate");
                candidate_line = 1;
            }
        }
    }
}

const DIFF_HALF_BOUND: usize = 4;
fn diff_half_universe<const N: usize>(
    first_line: u32,
) -> ([MutantId; N], [bool; N], [bool; N], [bool; N], Vec<MutantBaselineEntry>, MutantsBaseline) {
    let candidates: [MutantId; N] =
        core::array::from_fn(|index| build_kani_diff_candidate(first_line + index as u32));
    let survivor_selected: [bool; N] = kani::any();
    let baseline_selected: [bool; N] = kani::any();
    let baseline_expired: [bool; N] = kani::any();
    let entries: Vec<MutantBaselineEntry> = (0..N)
        .map(|index| MutantBaselineEntry {
            mutation_id: candidates[index].clone(),
            accepted_by_rule: diff_entry_rule(baseline_selected[index], baseline_expired[index]),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(baseline_selected[index], baseline_expired[index]),
        })
        .collect();
    let baseline = MutantsBaseline::from_bypasses(entries.clone());
    (candidates, survivor_selected, baseline_selected, baseline_expired, entries, baseline)
}
fn diff_half_battery<const N: usize>(
    candidates: &[MutantId; N],
    survivor_selected: &[bool; N],
    baseline_selected: &[bool; N],
    baseline_expired: &[bool; N],
    baseline_entries: &[MutantBaselineEntry],
    baseline: &MutantsBaseline,
) {
    let diff = baseline.diff(candidates, KANI_DIFF_NOW_UNIX);
    (0..N).for_each(|index| {
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
    diff.iter().for_each(|entry| {
        kani::assert(
            candidates.iter().any(|survivor| survivor == *entry),
            "diff never returns an identifier outside the survivor slice",
        );
    });
    diff_half_covers(
        candidates,
        survivor_selected,
        baseline_selected,
        baseline_expired,
        baseline_entries,
        baseline,
    );
}
fn diff_half_covers<const N: usize>(
    candidates: &[MutantId; N],
    survivor_selected: &[bool; N],
    baseline_selected: &[bool; N],
    baseline_expired: &[bool; N],
    baseline_entries: &[MutantBaselineEntry],
    baseline: &MutantsBaseline,
) {
    let any_covered_selected = (0..N)
        .any(|i| survivor_selected[i] && baseline.contains(&candidates[i], KANI_DIFF_NOW_UNIX));
    let any_uncovered_selected = (0..N)
        .any(|i| survivor_selected[i] && !baseline.contains(&candidates[i], KANI_DIFF_NOW_UNIX));
    let any_expired_entry = baseline_entries
        .iter()
        .any(|entry| entry.expires_on_unix.is_some_and(|expiry| expiry < KANI_DIFF_NOW_UNIX));
    let all_baseline_active = baseline_selected.iter().all(|selected| *selected)
        && baseline_expired.iter().all(|expired| !*expired);
    let all_survivors_selected = survivor_selected.iter().all(|selected| *selected);
    let no_survivor_selected = survivor_selected.iter().all(|selected| !*selected);
    kani::cover!(no_survivor_selected, "no-survivor selection is reachable");
    kani::cover!(all_survivors_selected, "all four survivors selected is reachable");
    kani::cover!(
        baseline_selected.iter().all(|selected| !*selected),
        "no baseline selection is reachable"
    );
    kani::cover!(
        all_baseline_active,
        "all four baseline entries active simultaneously is reachable"
    );
    kani::cover!(
        any_covered_selected && any_uncovered_selected,
        "covered and uncovered selected survivors coexist"
    );
    kani::cover!(any_expired_entry, "expired baseline entry is reachable");
}
#[kani::proof]
#[kani::unwind(16)]
fn kani_mutants_diff_first_half() {
    let (candidates, survivor_selected, baseline_selected, baseline_expired, entries, baseline) =
        diff_half_universe::<DIFF_HALF_BOUND>(1);
    diff_half_battery(
        &candidates,
        &survivor_selected,
        &baseline_selected,
        &baseline_expired,
        &entries,
        &baseline,
    );
}
#[kani::proof]
#[kani::unwind(16)]
fn kani_mutants_diff_second_half() {
    let (candidates, survivor_selected, baseline_selected, baseline_expired, entries, baseline) =
        diff_half_universe::<DIFF_HALF_BOUND>(1 + DIFF_HALF_BOUND as u32);
    diff_half_battery(
        &candidates,
        &survivor_selected,
        &baseline_selected,
        &baseline_expired,
        &entries,
        &baseline,
    );
}
#[kani::proof]
#[kani::unwind(16)]
fn kani_mutants_diff_all_active_fixed() {
    let candidates: [MutantId; KANI_DIFF_CANDIDATE_BOUND] =
        core::array::from_fn(|index| build_kani_diff_candidate(1 + index as u32));
    let entries: Vec<MutantBaselineEntry> = candidates
        .iter()
        .map(|candidate| MutantBaselineEntry {
            mutation_id: candidate.clone(),
            accepted_by_rule: diff_entry_rule(true, false),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(true, false),
        })
        .collect();
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = [true; KANI_DIFF_CANDIDATE_BOUND];
    let diff = baseline.diff(&candidates, KANI_DIFF_NOW_UNIX);
    kani::assert(diff.is_empty(), "fully covered universe diffs to empty");
    kani::cover!(survivors.iter().all(|selected| *selected), "all-selected is reachable");
}
#[kani::proof]
#[kani::unwind(16)]
fn kani_mutants_diff_mixed_boundary_fixed() {
    let candidates: [MutantId; KANI_DIFF_CANDIDATE_BOUND] =
        core::array::from_fn(|index| build_kani_diff_candidate(1 + index as u32));
    let entries: Vec<MutantBaselineEntry> = [1, 2, 3, 6]
        .into_iter()
        .map(|line| MutantBaselineEntry {
            mutation_id: build_kani_diff_candidate(line),
            accepted_by_rule: diff_entry_rule(true, line == 3),
            reason: DIFF_PROOF_REASON.to_owned(),
            expires_on_unix: diff_entry_expiry(true, line == 3),
        })
        .collect();
    let baseline = MutantsBaseline::from_bypasses(entries);
    let diff = baseline.diff(&candidates, KANI_DIFF_NOW_UNIX);
    let expected: [MutantId; 5] = [3, 4, 5, 7, 8].map(build_kani_diff_candidate);
    kani::assert(diff.len() == expected.len(), "mixed boundary diff retains five survivors");
    diff.iter().for_each(|entry| {
        kani::assert(
            expected.iter().any(|wanted| wanted == *entry),
            "mixed boundary diff holds only uncovered or expired survivors",
        );
    });
    expected.iter().for_each(|wanted| {
        kani::assert(
            diff.iter().any(|entry| *entry == wanted),
            "every uncovered or expired survivor is retained",
        );
    });
}

fn bounded_wire_string() -> String {
    let bytes: [u8; 8] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= bytes.len());
    match core::str::from_utf8(&bytes[..len]) {
        Ok(text) => text.to_owned(),
        Err(_) => String::from("wire"),
    }
}
#[kani::proof]
#[kani::unwind(32)]
fn outcome_wire_serialise_bounded() {
    let variant: u8 = kani::any();
    kani::assume(variant < 4);
    let message = bounded_wire_string();
    let tool = bounded_wire_string();
    let reason = bounded_wire_string();
    let version = bounded_wire_string();
    let outcome = match variant {
        0 => {
            let command = match CommandEvidence::new(
                String::from("cargo"),
                vec![String::from("cargo"), String::from("fmt")].into_boxed_slice(),
            ) {
                Ok(command) => command,
                Err(_) => {
                    kani::assert(false, "fixed command evidence must construct");
                    return;
                }
            };
            let evidence = match LaneEvidence::new(
                command,
                version,
                ProcessTermination::Exited { code: 0 },
                Digest::from_bytes(b"serialise-bound"),
            ) {
                Ok(evidence) => evidence,
                Err(_) => {
                    kani::assert(false, "fixed-shape clean evidence must construct");
                    return;
                }
            };
            LaneOutcome::Clean { evidence }
        }
        1 => {
            let path = match WorkspacePath::new("src/lib.rs") {
                Ok(path) => path,
                Err(_) => {
                    kani::assert(false, "fixed workspace path must parse");
                    return;
                }
            };
            let rule = match RuleId::new("FUNC_UNWRAP_USED") {
                Ok(rule) => rule,
                Err(_) => {
                    kani::assert(false, "fixed rule id must parse");
                    return;
                }
            };
            let location = match Location::span(path, 1, 0, 1, 5) {
                Ok(location) => location,
                Err(_) => {
                    kani::assert(false, "fixed span must construct");
                    return;
                }
            };
            let finding = Finding::reject(
                Lane::AstGrep,
                rule,
                location,
                message,
                RepairHint::use_iterator_pipeline(String::from("x")),
            );
            LaneOutcome::Findings { findings: Box::new([finding]) }
        }
        2 => LaneOutcome::Failed { failure: LaneFailure::Infra { tool, reason } },
        _ => LaneOutcome::Skipped { reason: SkipReason::NotSelectedByScope },
    };
    kani::cover!(variant == 0, "clean serialisation is reachable");
    kani::cover!(variant == 1, "findings serialisation is reachable");
    kani::cover!(variant == 2, "failed serialisation is reachable");
    kani::cover!(variant == 3, "skipped serialisation is reachable");
    match serde_json::to_string(&outcome) {
        Ok(serialised) => {
            kani::assert(!serialised.is_empty(), "serialised outcome is non-empty");
        }
        Err(_) => kani::assert(false, "serialise must succeed over constructed outcomes"),
    }
}
