//! Byte/codepoint-accurate span + column reporting for the ast-grep lane (H3).
//!
//! Black-hat finding H3: running the ast-grep lane on a demo file produced a
//! `FUNC_LOOPS_FOR` finding with `"line_start": 4, "col_start": 0` for a `for`
//! keyword indented four spaces, i.e. actually at column 4. v1-spec §10
//! mandates "1-based lines, 0-based columns (Unicode scalar values)" and sells
//! `TextRange`/`Location::Span` as enabling deterministic patching.
//!
//! These tests:
//! 1. Reproduce the bug on a focused fixture (`for` at line 4, col 4).
//! 2. Pin byte/codepoint-accurate columns for the H3 case.
//! 3. Round-trip every reported finding's start coordinate back against the
//!    original source — reading `(line_start-1, col_start)` MUST land on the
//!    first rune of the offending keyword. This is the deterministic-patching
//!    guarantee: the coordinate identifies the exact token.
//!
//! Multi-byte UTF-8 column math (2/3/4-byte leading runes, CJK matched token)
//! is covered exhaustively by unit tests in `src/ast_grep_lane/span.rs`, which
//! test the pure byte→(line,col) calculus directly. This integration test
//! covers the end-to-end lane → Location::Span path over real ast-grep parses.

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

/// `(line_start, col_start, line_end, col_end)` read from a serialized
/// `Location::Span`.
///
/// `Location` is a serde-transparent newtype over an externally-tagged enum,
/// so its JSON shape is `{"Span": {"file": ..., "line_start": N, ... }}`.
/// Reading the reported coordinates via serialization is the only public
/// channel because `Location` deliberately does not expose column accessors
/// (columns are write-only domain data by design).
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

/// Run the functional rule catalog against one fixture and return its findings.
fn run_one(fixture: &str) -> Vec<titania_core::Finding> {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let paths = vec![fixture_path(fixture)];
    match titania_lanes::ast_grep_lane::run(&rules_yaml, &paths, &[]) {
        Ok(LaneOutcome::Findings { findings }) => findings.into_vec(),
        Ok(other) => panic!("expected findings for {fixture}, got {other:?}"),
        Err(err) => panic!("lane error for {fixture}: {err}"),
    }
}

/// Index findings by rule id string for stable lookup.
fn by_rule_id(findings: &[titania_core::Finding]) -> BTreeMap<String, &titania_core::Finding> {
    findings.iter().map(|f| (f.rule_id().as_str().to_owned(), f)).collect()
}

/// Read the 1-based `line`, then split into 0-based Unicode scalar values
/// (Rust `char`s) and return the slice starting at `col`.
///
/// This is the deterministic-patching oracle: the coordinate reported by the
/// lane, read back out of the original source, must land on the offending
/// token's first rune.
fn line_from_coords(source: &str, line_1_based: u64) -> &str {
    source.lines().nth(usize::try_from(line_1_based).unwrap().saturating_sub(1)).unwrap_or("")
}

fn starts_with_token(line: &str, col_0_based: u64, token: &str) -> bool {
    let col = usize::try_from(col_0_based).unwrap();
    let chars: Vec<char> = line.chars().collect();
    let tail: String = chars.iter().skip(col).collect();
    tail.starts_with(token)
}

/// The reported start coordinate lands on a non-whitespace rune — i.e. it
/// identifies the start of an actual source token rather than indentation.
/// This is the weakest deterministic-patching guarantee and must hold for
/// every finding regardless of rule.
fn lands_on_token(line: &str, col_0_based: u64) -> bool {
    let col = usize::try_from(col_0_based).unwrap();
    line.chars().nth(col).is_some_and(|c| !c.is_whitespace())
}

// ===========================================================================
// H3 reproduction: `for` at 1-based line 4, 0-based column 4 (not 0).
// ===========================================================================

#[test]
fn h3_for_keyword_indented_four_spaces_reports_col_4_not_col_0() -> TestResult {
    let findings = run_one("functional/for_indented_col4_violation.rs");
    let by_id = by_rule_id(&findings);
    let finding =
        by_id.get("FUNC_LOOPS_FOR").copied().expect("fixture must trigger FUNC_LOOPS_FOR");

    let (line_start, col_start, line_end, col_end) = span_coords(finding);

    // The `for` keyword is on 1-based line 4 of the fixture, indented four
    // ASCII spaces, so its 0-based codepoint column is 4. The black-hat report
    // observed col_start == 0 here; this assertion pins the fix.
    assert_eq!(line_start, 4, "line_start must point at the `for` line");
    assert_eq!(col_start, 4, "col_start must be 4 (four leading spaces), was 0 before H3 fix");

    // The end coordinate must stay on or after the start coordinate's line.
    assert!(line_end >= line_start, "line_end must not precede line_start");

    // Round-trip: reading (line 4, col 4) out of the source lands on `for`.
    let source =
        std::fs::read_to_string(fixture_path("functional/for_indented_col4_violation.rs"))?;
    let line = line_from_coords(&source, line_start);
    assert!(
        starts_with_token(line, col_start, "for"),
        "deterministic-patching round-trip failed: line {line_start} col {col_start} of the source is `{line}`, expected it to start with `for`"
    );

    // The end column, read on the start line, must not overrun it; for a
    // multi-line span line_end > line_start is expected (the for-body), so we
    // only assert col_end ordering when the span is single-line.
    if line_end == line_start {
        assert!(col_end >= col_start, "same-line col_end must be >= col_start");
    }
    Ok(())
}

// ===========================================================================
// Before/after anchor on the original functional fixture (two for-loops).
// ===========================================================================

#[test]
fn for_loop_fixture_first_match_lands_on_for_keyword() -> TestResult {
    let findings = run_one("functional/func_loops_for_violation.rs");
    let by_id = by_rule_id(&findings);
    let finding = by_id.get("FUNC_LOOPS_FOR").copied().expect("FUNC_LOOPS_FOR must fire");
    let (line_start, col_start, _, _) = span_coords(finding);

    let source = std::fs::read_to_string(fixture_path("functional/func_loops_for_violation.rs"))?;
    let line = line_from_coords(&source, line_start);
    assert!(
        starts_with_token(line, col_start, "for"),
        "first FUNC_LOOPS_FOR match must land on `for`: line {line_start} is `{line}`, col {col_start}"
    );
    // The first `for` in this fixture is indented four spaces.
    assert_eq!(col_start, 4, "first `for` is four spaces indented");
    Ok(())
}

// ===========================================================================
// Round-trip matrix: every engine-detected functional finding's start
// coordinate must identify the offending keyword in the original source.
// This is the deterministic-patching guarantee across the rule family.
// ===========================================================================

/// One row of the round-trip matrix: fixture, rule id, and (when known) the
/// keyword the reported start coordinate must land on. `keyword == None`
/// means the rule matches a sub-expression (e.g. `unwrap_or`) whose node
/// root is the receiver, so only the non-whitespace guarantee is checked.
struct RoundTripRow {
    fixture: &'static str,
    rule: &'static str,
    keyword: Option<&'static str>,
}

fn round_trip_matrix() -> Vec<RoundTripRow> {
    vec![
        RoundTripRow {
            fixture: "functional/for_indented_col4_violation.rs",
            rule: "FUNC_LOOPS_FOR",
            keyword: Some("for"),
        },
        RoundTripRow {
            fixture: "functional/func_loops_for_violation.rs",
            rule: "FUNC_LOOPS_FOR",
            keyword: Some("for"),
        },
        RoundTripRow {
            fixture: "functional/func_loops_while_violation.rs",
            rule: "FUNC_LOOPS_WHILE",
            keyword: Some("while"),
        },
        RoundTripRow {
            fixture: "functional/func_loops_loop_violation.rs",
            rule: "FUNC_LOOPS_LOOP",
            keyword: Some("loop"),
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
        // `unwrap_or` matches the receiver expression, so only the
        // non-whitespace guarantee applies (the node root is the receiver,
        // not a fixed keyword).
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

        // Universal guarantee: the coordinate identifies a real token start.
        assert!(
            lands_on_token(line, col_start),
            "{} round-trip failed on {}: line {} is `{}`, col {} landed on whitespace",
            row.rule,
            row.fixture,
            line_start,
            line,
            col_start,
        );

        // Keyword-specific guarantee (when the matched node begins with a
        // known keyword token).
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

// ===========================================================================
// Negative guarantee: columns are never reported as a placeholder 0 for an
// indented token. Any keyword that is not at column 0 must report col_start > 0.
// ===========================================================================

#[test]
fn indented_while_loop_reports_nonzero_column() -> TestResult {
    let findings = run_one("functional/func_loops_while_violation.rs");
    let by_id = by_rule_id(&findings);
    let finding = by_id.get("FUNC_LOOPS_WHILE").copied().expect("FUNC_LOOPS_WHILE must fire");
    let (_, col_start, _, _) = span_coords(finding);
    assert!(col_start > 0, "indented `while` must not report col_start == 0");
    Ok(())
}
