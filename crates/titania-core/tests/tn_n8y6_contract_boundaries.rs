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

#[test]
fn lane_digest_accepts_passed_equal_to_scanned_and_preserves_all_fields() {
    let lane = lane_name("fmt");

    let digest = match LaneDigest::new(lane, ReceiptLaneExit::Clean, 7, 7, 2) {
        Ok(digest) => digest,
        Err(error) => panic!("passed == scanned must be accepted: {error}"),
    };

    assert_eq!(digest.lane().as_str(), "fmt");
    assert_eq!(digest.exit(), ReceiptLaneExit::Clean);
    assert_eq!(digest.scanned(), 7);
    assert_eq!(digest.passed(), 7);
    assert_eq!(digest.finding_count(), 2);
}

#[test]
fn lane_digest_accepts_zero_scanned_zero_passed() {
    let lane = lane_name("clippy");

    let digest = match LaneDigest::new(lane, ReceiptLaneExit::Violations, 0, 0, 0) {
        Ok(digest) => digest,
        Err(error) => panic!("zero scanned with zero passed must be accepted: {error}"),
    };

    assert_eq!(digest.scanned(), 0);
    assert_eq!(digest.passed(), 0);
    assert_eq!(digest.finding_count(), 0);
}

#[test]
fn lane_digest_rejects_passed_one_above_scanned_with_exact_counts() {
    let lane = lane_name("dylint");

    let error = match LaneDigest::new(lane, ReceiptLaneExit::Clean, 5, 6, 0) {
        Ok(_) => panic!("passed one above scanned must be rejected"),
        Err(error) => error,
    };

    match error {
        ReceiptError::PassedExceedsScanned { passed, scanned } => {
            assert_eq!(passed, 6);
            assert_eq!(scanned, 5);
        }
        other => panic!("expected PassedExceedsScanned, got: {other:?}"),
    }
}

#[test]
fn lane_digest_deserialization_rejects_passed_above_scanned() {
    let wire = serde_json::json!({
        "lane": "fmt",
        "exit": "clean",
        "scanned": 1,
        "passed": 2,
        "finding_count": 0
    });

    let error = match serde_json::from_value::<LaneDigest>(wire) {
        Ok(_) => panic!("wire value with passed above scanned must be rejected"),
        Err(error) => error,
    };

    assert!(
        error.to_string().contains("exceeds scanned"),
        "error must name the scanned-bound violation, got: {error}"
    );
}

#[test]
fn baseline_contains_covered_survivor_and_diff_excludes_it() {
    let covered = mutant_id("pkg-a", "src/a.rs", 3);
    let baseline = MutantsBaseline::from_bypasses(vec![baseline_entry(covered.clone(), None)]);
    let survivors = vec![covered];

    let is_covered = baseline.contains(&survivors[0], 1_000);
    let diff = baseline.diff(&survivors, 1_000);

    assert!(is_covered, "covered survivor must be contained");
    assert!(diff.is_empty(), "covered survivor must leave the diff, got: {diff:?}");
}

#[test]
fn baseline_diff_returns_identical_reference_for_uncovered_survivor() {
    let unrelated = mutant_id("pkg-a", "src/a.rs", 3);
    let baseline = MutantsBaseline::from_bypasses(vec![baseline_entry(unrelated, None)]);
    let survivors = vec![mutant_id("pkg-b", "src/b.rs", 4)];

    let is_covered = baseline.contains(&survivors[0], 1_000);
    let diff = baseline.diff(&survivors, 1_000);

    assert!(!is_covered, "uncovered survivor must not be contained");
    assert_eq!(diff.len(), 1, "uncovered survivor must appear once, got: {diff:?}");
    assert!(
        std::ptr::eq(diff[0], &survivors[0]),
        "diff must return the identical survivor reference"
    );
}

#[test]
fn baseline_expired_entry_covers_nothing_and_diff_retains_survivor() {
    let stale = mutant_id("pkg-s", "src/s.rs", 5);
    let baseline = MutantsBaseline::from_bypasses(vec![baseline_entry(stale.clone(), Some(4_999))]);
    let survivors = vec![stale];

    let is_covered = baseline.contains(&survivors[0], 5_000);
    let diff = baseline.diff(&survivors, 5_000);

    assert!(!is_covered, "expired entry must not contain its survivor");
    assert_eq!(diff.len(), 1, "expired entry must retain the survivor, got: {diff:?}");
}

#[test]
fn baseline_empty_contains_no_survivor() {
    let baseline = MutantsBaseline::empty();
    let survivors = vec![mutant_id("pkg-e", "src/e.rs", 1)];

    let is_covered = baseline.contains(&survivors[0], u64::MAX);
    let diff = baseline.diff(&survivors, u64::MAX);

    assert!(!is_covered, "empty baseline must contain nothing");
    assert_eq!(diff.len(), 1);
    assert!(std::ptr::eq(diff[0], &survivors[0]));
}

#[test]
fn receipt_period_accepts_equal_start_and_finish_with_preserved_times() {
    let period = match ReceiptPeriod::new(10, 10) {
        Ok(period) => period,
        Err(error) => panic!("equal start and finish must be accepted: {error}"),
    };

    assert_eq!(period.started_at(), 10);
    assert_eq!(period.finished_at(), 10);
}

#[test]
fn receipt_period_rejects_finish_one_below_start_with_exact_times() {
    let error = match ReceiptPeriod::new(10, 9) {
        Ok(_) => panic!("finish one below start must be rejected"),
        Err(error) => error,
    };

    match error {
        ReceiptError::FinishedBeforeStarted { started_at, finished_at } => {
            assert_eq!(started_at, 10);
            assert_eq!(finished_at, 9);
        }
        other => panic!("expected FinishedBeforeStarted, got: {other:?}"),
    }
}
