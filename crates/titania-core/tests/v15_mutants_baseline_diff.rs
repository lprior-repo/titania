//! v1.5 contract tests for `MutantsBaseline::diff` (set difference).

use titania_core::{MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline};

fn entry_for(mutation_id: &str) -> MutantBaselineEntry {
    // mutation_id is a fully-formed `<pkg>::<rel-path>:<line>:<col>:<operator>`
    // string; using the parser guarantees the typed field stays canonical.
    let mid = MutantId::parse(mutation_id).unwrap_or_else(|error| {
        panic!("test fixture id `{mutation_id}` must parse as a MutantId: {error}")
    });
    MutantBaselineEntry {
        mutation_id: mid,
        accepted_by_rule: "mutant-accept/owner-r/test reason/never".to_owned(),
        reason: "test reason".to_owned(),
        expires_on_unix: None,
    }
}

fn entry_zero(mutation_id: &str) -> MutantBaselineEntry {
    let _ = MutantOperator::EqualReplace;
    entry_for(mutation_id)
}

#[test]
fn diff_empty_baseline_returns_all_survivors() {
    let baseline = MutantsBaseline::empty();
    let survivors = vec![
        MutantId::new("pkg-a", "src/a.rs", 1, 1, MutantOperator::EqualReplace).unwrap(),
        MutantId::new("pkg-b", "src/b.rs", 1, 1, MutantOperator::EqualReplace).unwrap(),
    ];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert_eq!(diff.len(), 2);
    assert!(diff.iter().any(|s| s.package() == "pkg-a"));
    assert!(diff.iter().any(|s| s.package() == "pkg-b"));
}

#[test]
fn diff_full_baseline_returns_empty() {
    let entries = vec![
        entry_zero("pkg-a::src/a.rs:1:1:equal_replace"),
        entry_zero("pkg-b::src/b.rs:1:1:equal_replace"),
    ];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![
        MutantId::parse("pkg-a::src/a.rs:1:1:equal_replace").unwrap(),
        MutantId::parse("pkg-b::src/b.rs:1:1:equal_replace").unwrap(),
    ];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert!(diff.is_empty());
}

#[test]
fn diff_partial_baseline_returns_only_new() {
    let entries = vec![entry_zero("pkg-a::src/a.rs:1:1:equal_replace")];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![
        MutantId::parse("pkg-a::src/a.rs:1:1:equal_replace").unwrap(),
        MutantId::parse("pkg-b::src/b.rs:1:1:equal_replace").unwrap(),
        MutantId::parse("pkg-c::src/c.rs:1:1:equal_replace").unwrap(),
    ];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert_eq!(diff.len(), 2);
    assert!(!diff.iter().any(|s| s.package() == "pkg-a"));
    assert!(diff.iter().any(|s| s.package() == "pkg-b"));
    assert!(diff.iter().any(|s| s.package() == "pkg-c"));
}

#[test]
fn diff_never_returns_more_than_survivors() {
    let entries =
        (0..10).map(|i| entry_zero(&format!("pkg-{i}::src/file.rs:1:1:equal_replace"))).collect();
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec![
        MutantId::parse("pkg-a::src/a.rs:1:1:equal_replace").unwrap(),
        MutantId::parse("pkg-b::src/b.rs:1:1:equal_replace").unwrap(),
    ];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert_eq!(diff.len(), 2);
}

#[test]
fn diff_handles_empty_survivors() {
    let baseline = MutantsBaseline::empty();
    let survivors: Vec<MutantId> = vec![];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert!(diff.is_empty());
}

#[test]
fn diff_filters_unknown_mutation_id_forms_out() {
    // Construct a baseline entry that contains a synthesised id; an
    // unrecognised survivor of a totally different shape must report as a
    // diff without ever needing a string comparison.
    let entries = vec![entry_zero("pkg-a::src/a.rs:1:1:equal_replace")];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let synthesised =
        MutantId::new("pkg-z", "src/z.rs", 9, 9, MutantOperator::EqualReplace).unwrap();
    let survivors = vec![synthesised];
    let diff = baseline.diff(&survivors, u64::MAX);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].package(), "pkg-z");
}
