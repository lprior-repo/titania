//! v1.5 contract tests for the pure-core cargo-mutants outcomes and
//! per-mutant records parsers
//! (`crates/titania-core/src/mutants_outcomes.rs`).
//!
//! Mirrors the spec promise in `.beads/tn-7bq2.1/boundary-map.md`:
//! the parsers accept `&str`, return typed thiserror errors, tolerate
//! documented unknown top-level keys (forward compatibility for
//! cargo-mutants field evolution), reject malformed required shapes,
//! build typed `MutantId`s from per-mutant records, and perform no
//! I/O / time / env / process access.

use std::path::PathBuf;

use titania_core::{
    MUTANTS_OUTCOMES_MAX_ENTRIES, MUTANTS_RECORDS_MAX_ENTRIES, MutantOperator, MutantRecord,
    MutantsOutcomes, MutantsOutcomesError, MutantsRecords, OutcomeScenario, OutcomeSummary,
    RawPoint, RawSpan, relative_mutant_path,
};

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

fn read_fixture(name: &str) -> (String, String) {
    let path = fixture_path(name);
    let label = path.display().to_string();
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture `{}` unreadable: {error}", path.display()));
    (label, contents)
}

#[test]
fn outcomes_happy_path_counts_missed_mutants() {
    let (label, contents) = read_fixture("v15_mutants_outcomes_missed.json");
    let outcomes =
        MutantsOutcomes::parse_str(&contents, &label).expect("happy path outcomes must parse");
    assert_eq!(outcomes.outcomes.len(), 3);
    assert_eq!(outcomes.missed_count(), 1);
    assert_eq!(outcomes.missed, Some(1));
    assert_eq!(outcomes.total_mutants, Some(2));
    assert_eq!(outcomes.cargo_mutants_version.as_deref(), Some("27.0.0"));
}

#[test]
fn outcomes_baseline_only_loads_with_zero_missed() {
    let (label, contents) = read_fixture("v15_mutants_outcomes_baseline_only.json");
    let outcomes = MutantsOutcomes::parse_str(&contents, &label).expect("baseline-only must parse");
    assert_eq!(outcomes.outcomes.len(), 1);
    assert_eq!(outcomes.missed_count(), 0);
    let baseline = &outcomes.outcomes[0];
    assert!(
        matches!(&baseline.scenario, OutcomeScenario::Baseline(marker) if marker == "Baseline"),
        "got {:?}",
        baseline.scenario
    );
    assert!(matches!(baseline.summary, OutcomeSummary::Success));
}

#[test]
fn outcomes_scenario_discriminates_baseline_and_mutant() {
    let (label, contents) = read_fixture("v15_mutants_outcomes_missed.json");
    let outcomes = MutantsOutcomes::parse_str(&contents, &label).expect("parse");
    let baseline = &outcomes.outcomes[0];
    let unviable = &outcomes.outcomes[1];
    let missed = &outcomes.outcomes[2];
    assert!(
        matches!(&baseline.scenario, OutcomeScenario::Baseline(marker) if marker == "Baseline")
    );
    let OutcomeScenario::Mutant { mutant } = &unviable.scenario else {
        panic!("expected Mutant scenario, got {:?}", unviable.scenario);
    };
    assert_eq!(mutant.package, "titania-core");
    assert!(matches!(unviable.summary, OutcomeSummary::Unviable));
    let OutcomeScenario::Mutant { mutant } = &missed.scenario else {
        panic!("expected Mutant scenario, got {:?}", missed.scenario);
    };
    assert!(matches!(missed.summary, OutcomeSummary::MissedMutant));
    assert!(mutant.name.contains("replace == with !="));
}

#[test]
fn outcomes_empty_with_unknown_keys_parses() {
    let (label, contents) = read_fixture("v15_mutants_outcomes_empty_unknown.json");
    let outcomes = MutantsOutcomes::parse_str(&contents, &label)
        .expect("empty outcomes with unknown keys must parse");
    assert_eq!(outcomes.outcomes.len(), 0);
    assert_eq!(outcomes.missed, Some(0));
    assert_eq!(outcomes.missed_count(), 0);
}

#[test]
fn outcomes_malformed_returns_typed_parse_error() {
    let (label, contents) = read_fixture("v15_mutants_outcomes_malformed.json");
    let err = MutantsOutcomes::parse_str(&contents, &label).expect_err("malformed must fail");
    assert!(matches!(err, MutantsOutcomesError::OutcomesJsonParse { .. }), "got {err:?}");
}

#[test]
fn outcomes_rejects_non_object_root() {
    let err =
        MutantsOutcomes::parse_str("[1, 2, 3]", "<inline>").expect_err("array root must fail");
    assert!(matches!(err, MutantsOutcomesError::OutcomesJsonParse { .. }), "got {err:?}");
}

#[test]
fn outcomes_cap_constant_is_generous() {
    assert!(MUTANTS_OUTCOMES_MAX_ENTRIES >= 1024);
}

#[test]
fn records_happy_path_builds_typed_ids_for_four_genres() {
    let (label, contents) = read_fixture("v15_mutants_records_typed.json");
    let records = MutantsRecords::parse_str(&contents, &label).expect("records must parse");
    assert_eq!(records.as_slice().len(), 4);

    let binary = records
        .as_slice()
        .iter()
        .find(|record| record.name.contains("replace == with !="))
        .expect("binary equal-record must be present");
    assert_eq!(binary.classify_operator(), MutantOperator::EqualReplace);
    let typed = binary.typed_id(&label).expect("binary record must build a typed id");
    assert!(typed.as_str().contains("artifact.rs:99:9:equal_replace"));

    let and_or = records
        .as_slice()
        .iter()
        .find(|record| record.name.contains("replace && with ||"))
        .expect("and-or record must be present");
    assert_eq!(and_or.classify_operator(), MutantOperator::AndOr);

    let unary = records
        .as_slice()
        .iter()
        .find(|record| record.name.contains("remove negation"))
        .expect("unary record must be present");
    assert_eq!(unary.classify_operator(), MutantOperator::RemoveNegation);

    let fn_value = records
        .as_slice()
        .iter()
        .find(|record| record.name.contains("replace ArtifactOutcome::default"))
        .expect("FnValue record must be present");
    assert_eq!(fn_value.classify_operator(), MutantOperator::DefaultReplace);
}

#[test]
fn records_round_trip_preserves_typed_id() {
    let (label, contents) = read_fixture("v15_mutants_records_typed.json");
    let records = MutantsRecords::parse_str(&contents, &label).expect("records must parse");
    let first = &records.as_slice()[0];
    let id = first.typed_id(&label).expect("typed id must build");
    let json = serde_json::to_string(first).expect("serialize must succeed");
    let back: MutantRecord = serde_json::from_str(&json).expect("round-trip must succeed");
    assert_eq!(back.classify_operator(), first.classify_operator());
    let back_id = back.typed_id(&label).expect("typed id after round-trip must build");
    assert_eq!(back_id, id);
}

#[test]
fn records_missing_span_returns_typed_error() {
    let (label, contents) = read_fixture("v15_mutants_records_missing_span.json");
    let records = MutantsRecords::parse_str(&contents, &label).expect("records must parse");
    let only = &records.as_slice()[0];
    let err = only.typed_id(&label).expect_err("missing span must fail");
    assert!(matches!(err, MutantsOutcomesError::MissingSourceSpan { .. }), "got {err:?}");
}

#[test]
fn records_file_outside_package_returns_typed_error() {
    let (label, contents) = read_fixture("v15_mutants_records_path_outside_package.json");
    let records = MutantsRecords::parse_str(&contents, &label).expect("records must parse");
    let only = &records.as_slice()[0];
    let err = only.typed_id(&label).expect_err("file outside package must fail");
    assert!(matches!(err, MutantsOutcomesError::PathOutsidePackage { .. }), "got {err:?}");
}

#[test]
fn records_malformed_returns_typed_parse_error() {
    let (label, contents) = read_fixture("v15_mutants_records_malformed.json");
    let err = MutantsRecords::parse_str(&contents, &label).expect_err("malformed must fail");
    assert!(matches!(err, MutantsOutcomesError::RecordsJsonParse { .. }), "got {err:?}");
}

#[test]
fn records_rejects_object_root() {
    let err = MutantsRecords::parse_str("{ \"not\": \"a list\" }", "<inline>")
        .expect_err("object root must fail");
    assert!(matches!(err, MutantsOutcomesError::RecordsJsonParse { .. }), "got {err:?}");
}

#[test]
fn records_cap_constant_is_generous() {
    assert!(MUTANTS_RECORDS_MAX_ENTRIES >= 1024);
}

#[test]
fn records_start_point_returns_none_when_span_missing() {
    let record = MutantRecord {
        name: String::from("name"),
        package: String::from("pkg"),
        file: String::from("crates/pkg/src/lib.rs"),
        span: None,
        genre: None,
        replacement: None,
        function: None,
    };
    assert!(record.start_point().is_none());
}

#[test]
fn records_start_point_returns_none_when_start_missing() {
    let record = MutantRecord {
        name: String::from("name"),
        package: String::from("pkg"),
        file: String::from("crates/pkg/src/lib.rs"),
        span: Some(RawSpan { start: None, end: None }),
        genre: None,
        replacement: None,
        function: None,
    };
    assert!(record.start_point().is_none());
}

#[test]
fn records_start_point_returns_line_and_column() {
    let record = MutantRecord {
        name: String::from("name"),
        package: String::from("pkg"),
        file: String::from("crates/pkg/src/lib.rs"),
        span: Some(RawSpan { start: Some(RawPoint { line: 42, column: 7 }), end: None }),
        genre: None,
        replacement: None,
        function: None,
    };
    assert_eq!(record.start_point(), Some((42, 7)));
}

#[test]
fn records_classify_operator_defaults_when_genre_missing() {
    let record = MutantRecord {
        name: String::from("crates/pkg/src/lib.rs:1:1: replace foo"),
        package: String::from("pkg"),
        file: String::from("crates/pkg/src/lib.rs"),
        span: None,
        genre: None,
        replacement: None,
        function: None,
    };
    assert_eq!(record.classify_operator(), MutantOperator::DefaultReplace);
}

#[test]
fn records_classify_operator_handles_unknown_binary_name() {
    let record = MutantRecord {
        name: String::from("crates/pkg/src/lib.rs:1:1: replace something else"),
        package: String::from("pkg"),
        file: String::from("crates/pkg/src/lib.rs"),
        span: None,
        genre: Some(String::from("BinaryOperator")),
        replacement: None,
        function: None,
    };
    assert_eq!(record.classify_operator(), MutantOperator::ArithmeticOpFlip);
}

#[test]
fn relative_mutant_path_strips_crates_prefix() {
    let path = relative_mutant_path("titania-core", "crates/titania-core/src/lib.rs")
        .expect("crates path inside package must succeed");
    assert_eq!(path, "src/lib.rs");
}

#[test]
fn relative_mutant_path_strips_dot_slash() {
    let path =
        relative_mutant_path("titania-core", "./src/lib.rs").expect("dot-slash path must succeed");
    assert_eq!(path, "src/lib.rs");
}

#[test]
fn relative_mutant_path_passes_bare_path_through() {
    let path = relative_mutant_path("titania-core", "src/lib.rs").expect("bare path must succeed");
    assert_eq!(path, "src/lib.rs");
}

#[test]
fn relative_mutant_path_rejects_other_crate() {
    assert!(relative_mutant_path("titania-core", "crates/titania-other/src/lib.rs").is_none());
}

#[test]
fn serde_round_trip_preserves_outcomes() {
    let (label, contents) = read_fixture("v15_mutants_outcomes_missed.json");
    let outcomes = MutantsOutcomes::parse_str(&contents, &label).expect("parse");
    let json = serde_json::to_string(&outcomes).expect("serialize must succeed");
    let back: MutantsOutcomes = serde_json::from_str(&json).expect("round-trip must succeed");
    assert_eq!(back, outcomes);
}

#[test]
fn serde_round_trip_preserves_records() {
    let (label, contents) = read_fixture("v15_mutants_records_typed.json");
    let records = MutantsRecords::parse_str(&contents, &label).expect("parse");
    let json = serde_json::to_string(&records).expect("serialize must succeed");
    let back: MutantsRecords = serde_json::from_str(&json).expect("round-trip must succeed");
    assert_eq!(back, records);
}
