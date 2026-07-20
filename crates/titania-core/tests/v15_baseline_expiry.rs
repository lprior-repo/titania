//! v1.5 contract tests for baseline expiry semantics.

use titania_core::{MutantBaselineEntry, MutantId, MutantsBaseline};

fn entry_for(mutation_id: &str, expires_on_unix: Option<u64>) -> MutantBaselineEntry {
    let mid = MutantId::parse(mutation_id).unwrap_or_else(|error| {
        panic!("test fixture id `{mutation_id}` must parse as a MutantId: {error}")
    });
    MutantBaselineEntry {
        mutation_id: mid,
        accepted_by_rule: "mutant-accept/owner-r/test reason/never".to_owned(),
        reason: "test reason".to_owned(),
        expires_on_unix,
    }
}

#[test]
fn expired_entry_does_not_cover() {
    let entries = vec![entry_for("expired::src/expired.rs:1:1:equal_replace", Some(1_000))];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![MutantId::parse("expired::src/expired.rs:1:1:equal_replace").unwrap()];
    let diff = baseline.diff(&survivors, 2_000);
    assert_eq!(diff.len(), 1, "expired entries must NOT suppress findings");
}

#[test]
fn unexpired_entry_covers() {
    let entries = vec![entry_for("fresh::src/fresh.rs:1:1:equal_replace", Some(10_000))];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![MutantId::parse("fresh::src/fresh.rs:1:1:equal_replace").unwrap()];
    let diff = baseline.diff(&survivors, 5_000);
    assert!(diff.is_empty());
}

#[test]
fn boundary_timestamp_is_inclusive() {
    let entries = vec![entry_for("boundary::src/boundary.rs:1:1:equal_replace", Some(5_000))];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![MutantId::parse("boundary::src/boundary.rs:1:1:equal_replace").unwrap()];
    let diff = baseline.diff(&survivors, 5_000);
    assert!(diff.is_empty(), "now_unix == expires_on_unix must still cover");
}

#[test]
fn no_expiry_always_covers() {
    let entries = vec![entry_for("no-expiry::src/no-expiry.rs:1:1:equal_replace", None)];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![MutantId::parse("no-expiry::src/no-expiry.rs:1:1:equal_replace").unwrap()];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert!(diff.is_empty());
}
