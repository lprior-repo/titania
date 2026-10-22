use std::{
    collections::BTreeMap,
    error::Error,
    path::{Path, PathBuf},
};

use serde_json::Value;
use titania_core::LaneOutcome;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_path(name: &str) -> PathBuf {
    let base = env!("CARGO_MANIFEST_DIR");
    Path::new(base).join("tests").join("fixtures").join("ast_grep").join(name)
}

fn span_coords(finding: &titania_core::Finding) -> (u64, u64, u64, u64) {
    let value = serde_json::to_value(finding.location()).expect("location must serialize");
    let span = value
        .get("Span")
        .expect("ast-grep findings must use Location::Span, got a different variant");
    let line_start = span.get("line_start").and_then(Value::as_u64).expect("line_start is a u32");
    let col_start = span.get("col_start").and_then(Value::as_u64).expect("col_start is a u32");
    let line_end = span.get("line_end").and_then(Value::as_u64).expect("line_end is a u32");
    let col_end = span.get("col_end").and_then(Value::as_u64).expect("col_end is a u32");
    (line_start, col_start, line_end, col_end)
}

fn run_one(fixture: &str) -> Vec<titania_core::Finding> {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let paths = vec![fixture_path(fixture)];
    match titania_lanes::ast_grep_lane::run(&rules_yaml, &paths, &[]) {
        Ok(LaneOutcome::Findings { findings }) => findings.into_vec(),
        Ok(other) => panic!("expected findings for {fixture}, got {other:?}"),
        Err(err) => panic!("lane error for {fixture}: {err}"),
    }
}

fn by_rule_id(findings: &[titania_core::Finding]) -> BTreeMap<String, &titania_core::Finding> {
    let mut by_id: BTreeMap<String, &titania_core::Finding> = BTreeMap::new();
    for finding in findings {
        if !by_id.contains_key(finding.rule_id().as_str()) {
            let _previous = by_id.insert(finding.rule_id().as_str().to_owned(), finding);
        }
    }
    by_id
}

fn line_from_coords(source: &str, line_1_based: u64) -> &str {
    source.lines().nth(usize::try_from(line_1_based).unwrap().saturating_sub(1)).unwrap_or("")
}

fn starts_with_token(line: &str, col_0_based: u64, token: &str) -> bool {
    let col = usize::try_from(col_0_based).unwrap();
    let chars: Vec<char> = line.chars().collect();
    let tail: String = chars.iter().skip(col).collect();
    tail.starts_with(token)
}

fn lands_on_token(line: &str, col_0_based: u64) -> bool {
    let col = usize::try_from(col_0_based).unwrap();
    line.chars().nth(col).is_some_and(|c| !c.is_whitespace())
}

#[test]
fn h3_for_keyword_indented_four_spaces_reports_col_4_not_col_0() -> TestResult {
    let findings = run_one("functional/loop_indented_col4_violation.rs");
    let by_id = by_rule_id(&findings);
    let finding = by_id
        .get("BOUNDED_EXECUTION_UNBOUNDED_LOOP")
        .copied()
        .expect("fixture must trigger BOUNDED_EXECUTION_UNBOUNDED_LOOP");

    let (line_start, col_start, line_end, col_end) = span_coords(finding);

    assert_eq!(line_start, 3, "line_start must point at the `loop` line");
    assert_eq!(col_start, 4, "col_start must be 4 (four leading spaces), was 0 before H3 fix");

    assert!(line_end >= line_start, "line_end must not precede line_start");

    let source =
        std::fs::read_to_string(fixture_path("functional/loop_indented_col4_violation.rs"))?;
    let line = line_from_coords(&source, line_start);
    assert!(
        starts_with_token(line, col_start, "loop"),
        "deterministic-patching round-trip failed: line {line_start} col {col_start} of the source is `{line}`, expected it to start with `loop`"
    );

    if line_end == line_start {
        assert!(col_end >= col_start, "same-line col_end must be >= col_start");
    }
    Ok(())
}

#[test]
fn for_loop_fixture_first_match_lands_on_for_keyword() -> TestResult {
    let findings = run_one("functional/bounded_execution_unbounded_loop_violation.rs");
    let by_id = by_rule_id(&findings);
    let finding = by_id
        .get("BOUNDED_EXECUTION_UNBOUNDED_LOOP")
        .copied()
        .expect("BOUNDED_EXECUTION_UNBOUNDED_LOOP must fire");
    let (line_start, col_start, _, _) = span_coords(finding);

    let source = std::fs::read_to_string(fixture_path(
        "functional/bounded_execution_unbounded_loop_violation.rs",
    ))?;
    let line = line_from_coords(&source, line_start);
    assert!(
        starts_with_token(line, col_start, "loop"),
        "first BOUNDED_EXECUTION_UNBOUNDED_LOOP match must land on `loop`: line {line_start} is `{line}`, col {col_start}"
    );
    assert_eq!(col_start, 4, "first `loop` is four spaces indented");
    Ok(())
}

struct RoundTripRow {
    fixture: &'static str,
    rule: &'static str,
    keyword: Option<&'static str>,
}

fn round_trip_matrix() -> Vec<RoundTripRow> {
    vec![
        RoundTripRow {
            fixture: "functional/loop_indented_col4_violation.rs",
            rule: "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
            keyword: Some("loop"),
        },
        RoundTripRow {
            fixture: "functional/bounded_execution_unbounded_loop_violation.rs",
            rule: "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
            keyword: Some("loop"),
        },
        RoundTripRow {
            fixture: "functional/bounded_execution_unbounded_while_violation.rs",
            rule: "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
            keyword: Some("while"),
        },
        RoundTripRow {
            fixture: "functional/bounded_execution_unbounded_iterator_violation.rs",
            rule: "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR",
            keyword: None,
        },
        RoundTripRow {
            fixture: "functional/func_print_stdout_violation.rs",
            rule: "FUNC_PRINT_STDOUT",
            keyword: Some("println!"),
        },
        RoundTripRow {
            fixture: "functional/func_print_stderr_violation.rs",
            rule: "FUNC_PRINT_STDERR",
            keyword: Some("eprintln!"),
        },
        RoundTripRow {
            fixture: "functional/func_wildcard_import_violation.rs",
            rule: "FUNC_WILDCARD_IMPORT",
            keyword: Some("use"),
        },
        RoundTripRow {
            fixture: "functional/func_unwrap_or_violation.rs",
            rule: "FUNC_UNWRAP_OR",
            keyword: None,
        },
    ]
}

#[test]
fn every_functional_finding_start_coordinate_lands_on_the_offending_token() -> TestResult {
    for row in round_trip_matrix() {
        let findings = run_one(row.fixture);
        let by_id = by_rule_id(&findings);
        let finding = by_id
            .get(row.rule)
            .copied()
            .unwrap_or_else(|| panic!("{} must fire on {}", row.rule, row.fixture));
        let (line_start, col_start, _, _) = span_coords(finding);

        let source = std::fs::read_to_string(fixture_path(row.fixture))?;
        let line = line_from_coords(&source, line_start);

        assert!(
            lands_on_token(line, col_start),
            "{} round-trip failed on {}: line {} is `{}`, col {} landed on whitespace",
            row.rule,
            row.fixture,
            line_start,
            line,
            col_start,
        );

        if let Some(keyword) = row.keyword {
            assert!(
                starts_with_token(line, col_start, keyword),
                "{} round-trip failed on {}: line {} is `{}`, col {} should start with `{}`",
                row.rule,
                row.fixture,
                line_start,
                line,
                col_start,
                keyword,
            );
        }
    }
    Ok(())
}

#[test]
fn indented_while_loop_reports_nonzero_column() -> TestResult {
    let findings = run_one("functional/bounded_execution_unbounded_while_violation.rs");
    let by_id = by_rule_id(&findings);
    let finding = by_id
        .get("BOUNDED_EXECUTION_UNBOUNDED_LOOP")
        .copied()
        .expect("BOUNDED_EXECUTION_UNBOUNDED_LOOP must fire");
    let (_, col_start, _, _) = span_coords(finding);
    assert!(col_start > 0, "indented `while` must not report col_start == 0");
    Ok(())
}
