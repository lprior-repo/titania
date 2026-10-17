//! Behavior contract boundaries for bead `tn-n8y6`.
//!
//! Proves selected consumer-visible boundary and error behavior through the
//! real `titania-core` production APIs with exact semantic assertions:
//!
//! - `LaneDigest::new` accepts the `passed == scanned` and `(0, 0)`
//!   boundaries with all fields preserved, and rejects `passed > scanned`
//!   with the exact `PassedExceedsScanned { passed, scanned }` payload —
//!   including through the serde boundary.
//! - `MutantsBaseline::contains` / `diff` move survivors across the
//!   covered/uncovered transition: covered survivors are contained and
//!   excluded from the diff, uncovered survivors are uncontained and the
//!   diff returns the identical survivor reference, expired entries cover
//!   nothing, and an empty baseline contains nothing.
//! - `ReceiptPeriod::new` accepts the `started_at == finished_at` boundary
//!   with times preserved, and rejects `finished_at == started_at - 1` with
//!   the exact `FinishedBeforeStarted` payload.
//!
//! Kani harnesses (`crates/titania-core/src/kani.rs`) prove the same
//! constructors symbolically; these executable tests are the behavior
//! evidence and would fail if the production behavior were deleted.

use titania_core::{
    LaneDigest, LaneName, MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline,
    ReceiptError, ReceiptLaneExit, ReceiptPeriod,
};

fn lane_name(value: &str) -> LaneName {
    match LaneName::new(value) {
        Ok(name) => name,
        Err(error) => panic!("fixture lane name `{value}` must validate: {error}"),
    }
}

fn mutant_id(package: &str, path: &str, line: u32) -> MutantId {
    match MutantId::new(package, path, line, 1, MutantOperator::EqualReplace) {
        Ok(id) => id,
        Err(error) => panic!("fixture mutant id must validate: {error}"),
    }
}

fn baseline_entry(mutation_id: MutantId, expires_on_unix: Option<u64>) -> MutantBaselineEntry {
    MutantBaselineEntry {
        mutation_id,
        accepted_by_rule: "mutant-accept/owner-a/reason-a/never".to_owned(),
        reason: "tn-n8y6 boundary probe".to_owned(),
        expires_on_unix,
    }
}

// ---------------------------------------------------------------------------
// LaneDigest boundary: passed == scanned is accepted with fields preserved.
// ---------------------------------------------------------------------------

#[test]
fn lane_digest_accepts_passed_equal_to_scanned_and_preserves_all_fields() {
    // Given: the exact equality boundary value.
    let lane = lane_name("fmt");

    // When: the real production constructor runs.
    let digest = match LaneDigest::new(lane, ReceiptLaneExit::Clean, 7, 7, 2) {
        Ok(digest) => digest,
        Err(error) => panic!("passed == scanned must be accepted: {error}"),
    };

    // Then: every field is preserved exactly.
    assert_eq!(digest.lane().as_str(), "fmt");
    assert_eq!(digest.exit(), ReceiptLaneExit::Clean);
    assert_eq!(digest.scanned(), 7);
    assert_eq!(digest.passed(), 7);
    assert_eq!(digest.finding_count(), 2);
}

#[test]
fn lane_digest_accepts_zero_scanned_zero_passed() {
    // Given: the degenerate zero boundary value.
    let lane = lane_name("clippy");

    // When: the real production constructor runs.
    let digest = match LaneDigest::new(lane, ReceiptLaneExit::Violations, 0, 0, 0) {
        Ok(digest) => digest,
        Err(error) => panic!("zero scanned with zero passed must be accepted: {error}"),
    };

    // Then: the zero counts round-trip exactly.
    assert_eq!(digest.scanned(), 0);
    assert_eq!(digest.passed(), 0);
    assert_eq!(digest.finding_count(), 0);
}

// ---------------------------------------------------------------------------
// LaneDigest error: passed one above scanned is rejected with exact payload.
// ---------------------------------------------------------------------------

#[test]
fn lane_digest_rejects_passed_one_above_scanned_with_exact_counts() {
    // Given: a passed count exactly one above the scanned boundary.
    let lane = lane_name("dylint");

    // When: the real production constructor runs.
    let error = match LaneDigest::new(lane, ReceiptLaneExit::Clean, 5, 6, 0) {
        Ok(_) => panic!("passed one above scanned must be rejected"),
        Err(error) => error,
    };

    // Then: the rejection names the exact offending counts.
    match error {
        ReceiptError::PassedExceedsScanned { passed, scanned } => {
            assert_eq!(passed, 6);
            assert_eq!(scanned, 5);
        }
        other => panic!("expected PassedExceedsScanned, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// LaneDigest serde boundary: missing validity check fails deserialization.
// ---------------------------------------------------------------------------

#[test]
fn lane_digest_deserialization_rejects_passed_above_scanned() {
    // Given: a wire value that violates the passed <= scanned contract.
    let wire = serde_json::json!({
        "lane": "fmt",
        "exit": "clean",
        "scanned": 1,
        "passed": 2,
        "finding_count": 0
    });

    // When: the production Deserialize boundary runs.
    let error = match serde_json::from_value::<LaneDigest>(wire) {
        Ok(_) => panic!("wire value with passed above scanned must be rejected"),
        Err(error) => error,
    };

    // Then: the failure names the contract violation.
    assert!(
        error.to_string().contains("exceeds scanned"),
        "error must name the scanned-bound violation, got: {error}"
    );
}

// ---------------------------------------------------------------------------
// Baseline transition: covered survivor is contained and leaves the diff.
// ---------------------------------------------------------------------------

#[test]
fn baseline_contains_covered_survivor_and_diff_excludes_it() {
    // Given: a baseline covering exactly one survivor.
    let covered = mutant_id("pkg-a", "src/a.rs", 3);
    let baseline = MutantsBaseline::from_bypasses(vec![baseline_entry(covered.clone(), None)]);
    let survivors = vec![covered];

    // When: the production contains/diff transition runs.
    let is_covered = baseline.contains(&survivors[0], 1_000);
    let diff = baseline.diff(&survivors, 1_000);

    // Then: the survivor is contained and produces no finding.
    assert!(is_covered, "covered survivor must be contained");
    assert!(diff.is_empty(), "covered survivor must leave the diff, got: {diff:?}");
}

// ---------------------------------------------------------------------------
// Baseline transition: uncovered survivor keeps its identity in the diff.
// ---------------------------------------------------------------------------

#[test]
fn baseline_diff_returns_identical_reference_for_uncovered_survivor() {
    // Given: a baseline covering a different mutant than the survivor.
    let unrelated = mutant_id("pkg-a", "src/a.rs", 3);
    let baseline = MutantsBaseline::from_bypasses(vec![baseline_entry(unrelated, None)]);
    let survivors = vec![mutant_id("pkg-b", "src/b.rs", 4)];

    // When: the production contains/diff transition runs.
    let is_covered = baseline.contains(&survivors[0], 1_000);
    let diff = baseline.diff(&survivors, 1_000);

    // Then: the survivor is uncontained and the diff yields the very same
    // reference (no clone, no synthesized id).
    assert!(!is_covered, "uncovered survivor must not be contained");
    assert_eq!(diff.len(), 1, "uncovered survivor must appear once, got: {diff:?}");
    assert!(
        std::ptr::eq(diff[0], &survivors[0]),
        "diff must return the identical survivor reference"
    );
}

// ---------------------------------------------------------------------------
// Baseline error path: an expired entry covers nothing and retains the diff.
// ---------------------------------------------------------------------------

#[test]
fn baseline_expired_entry_covers_nothing_and_diff_retains_survivor() {
    // Given: an entry whose expiry is strictly before now.
    let stale = mutant_id("pkg-s", "src/s.rs", 5);
    let baseline = MutantsBaseline::from_bypasses(vec![baseline_entry(stale.clone(), Some(4_999))]);
    let survivors = vec![stale];

    // When: the production contains/diff transition runs at now = 5_000.
    let is_covered = baseline.contains(&survivors[0], 5_000);
    let diff = baseline.diff(&survivors, 5_000);

    // Then: the expired entry suppresses nothing.
    assert!(!is_covered, "expired entry must not contain its survivor");
    assert_eq!(diff.len(), 1, "expired entry must retain the survivor, got: {diff:?}");
}

// ---------------------------------------------------------------------------
// Baseline empty boundary: an empty baseline contains nothing.
// ---------------------------------------------------------------------------

#[test]
fn baseline_empty_contains_no_survivor() {
    // Given: an empty baseline and one survivor.
    let baseline = MutantsBaseline::empty();
    let survivors = vec![mutant_id("pkg-e", "src/e.rs", 1)];

    // When: the production contains/diff transition runs.
    let is_covered = baseline.contains(&survivors[0], u64::MAX);
    let diff = baseline.diff(&survivors, u64::MAX);

    // Then: nothing is contained and the survivor is reported.
    assert!(!is_covered, "empty baseline must contain nothing");
    assert_eq!(diff.len(), 1);
    assert!(std::ptr::eq(diff[0], &survivors[0]));
}

// ---------------------------------------------------------------------------
// ReceiptPeriod boundary: equal start/finish is accepted with times kept.
// ---------------------------------------------------------------------------

#[test]
fn receipt_period_accepts_equal_start_and_finish_with_preserved_times() {
    // Given: the exact simultaneity boundary value.

    // When: the real production constructor runs.
    let period = match ReceiptPeriod::new(10, 10) {
        Ok(period) => period,
        Err(error) => panic!("equal start and finish must be accepted: {error}"),
    };

    // Then: both timestamps are preserved exactly.
    assert_eq!(period.started_at(), 10);
    assert_eq!(period.finished_at(), 10);
}

// ---------------------------------------------------------------------------
// ReceiptPeriod error: finish one below start is rejected with exact times.
// ---------------------------------------------------------------------------

#[test]
fn receipt_period_rejects_finish_one_below_start_with_exact_times() {
    // Given: a finish timestamp exactly one below the start boundary.

    // When: the real production constructor runs.
    let error = match ReceiptPeriod::new(10, 9) {
        Ok(_) => panic!("finish one below start must be rejected"),
        Err(error) => error,
    };

    // Then: the rejection preserves the exact offending timestamps.
    match error {
        ReceiptError::FinishedBeforeStarted { started_at, finished_at } => {
            assert_eq!(started_at, 10);
            assert_eq!(finished_at, 9);
        }
        other => panic!("expected FinishedBeforeStarted, got: {other:?}"),
    }
}
