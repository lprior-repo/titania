#![allow(clippy::as_conversions)]
#![allow(clippy::useless_vec)]
#![allow(clippy::arithmetic_side_effects)]

use titania_core::{Location, LocationError, WorkspacePath};

fn file() -> WorkspacePath {
    WorkspacePath::new("src/lib.rs").expect("valid workspace path")
}

#[test]
fn location_span_accepts_cross_line_with_descending_columns() {
    let result = Location::span(file(), 10, 25, 12, 4);
    assert!(result.is_ok(), "cross-line span must accept descending columns, got: {result:?}");
}

#[test]
fn location_span_accepts_cross_line_with_zero_columns() {
    let result = Location::span(file(), 10, 0, 11, 0);
    assert!(
        result.is_ok(),
        "cross-line span with equal zero columns must be accepted, got: {result:?}"
    );
}

#[test]
fn location_span_accepts_cross_line_equal_columns() {
    let result = Location::span(file(), 5, 7, 9, 7);
    assert!(result.is_ok(), "cross-line span with equal columns must be accepted, got: {result:?}");
}

#[test]
fn location_span_accepts_cross_line_with_large_columns() {
    let result = Location::span(file(), 1, 0, 3, u32::MAX);
    assert!(
        result.is_ok(),
        "cross-line span with large end column must be accepted, got: {result:?}"
    );
}

#[test]
fn location_span_accepts_same_line_ordered_columns() {
    let result = Location::span(file(), 10, 5, 10, 30);
    assert!(result.is_ok(), "same-line ordered span must be accepted, got: {result:?}");
}

#[test]
fn location_span_accepts_same_line_zero_width() {
    let result = Location::span(file(), 10, 5, 10, 5);
    assert!(result.is_ok(), "same-line zero-width span must be accepted, got: {result:?}");
}

#[test]
fn location_span_rejects_same_line_descending_columns() {
    let result = Location::span(file(), 10, 25, 10, 4);
    assert!(result.is_err(), "same-line descending columns must be rejected, got: {result:?}");
    assert!(matches!(result, Err(LocationError::ColEndBeforeStart { col_start: 25, col_end: 4 })));
}

#[test]
fn location_span_rejects_same_line_descending_columns_zero_width() {
    let result = Location::span(file(), 10, 5, 10, 4);
    assert!(result.is_err(), "same-line col_end < col_start must be rejected, got: {result:?}");
    assert!(matches!(result, Err(LocationError::ColEndBeforeStart { col_start: 5, col_end: 4 })));
}

#[test]
fn location_span_rejects_line_end_before_line_start() {
    let result = Location::span(file(), 12, 0, 10, 0);
    assert!(result.is_err(), "line_end < line_start must be rejected, got: {result:?}");
    assert!(matches!(result, Err(LocationError::EndBeforeStart { line_start: 12, line_end: 10 })));
}

#[test]
fn location_span_rejects_line_start_zero() {
    let result = Location::span(file(), 0, 0, 1, 1);
    assert!(result.is_err(), "line_start < 1 must be rejected, got: {result:?}");
    assert!(matches!(result, Err(LocationError::LineStartBeforeOne)));
}

#[test]
fn location_span_rejects_line_start_zero_with_descending_cross_line_columns() {
    let result = Location::span(file(), 0, 25, 5, 4);
    assert!(result.is_err(), "line_start < 1 must be rejected first, got: {result:?}");
    assert!(matches!(result, Err(LocationError::LineStartBeforeOne)));
}

#[test]
fn location_span_line_invariant_takes_precedence_over_column_invariant() {
    let result = Location::span(file(), 10, 25, 5, 4);
    assert!(result.is_err(), "inverted lines must be rejected, got: {result:?}");
    assert!(
        matches!(result, Err(LocationError::EndBeforeStart { line_start: 10, line_end: 5 })),
        "line invariant must be checked before column invariant, got: {result:?}"
    );
}

const CROSS_LINE_SPAN_JSON: &str =
    r#"{"Span":{"file":"src/lib.rs","line_start":10,"col_start":25,"line_end":12,"col_end":4}}"#;

#[test]
fn location_deserializes_cross_line_span_with_descending_columns() {
    let parsed: Location =
        serde_json::from_str(CROSS_LINE_SPAN_JSON).expect("cross-line span must deserialize");
    let expected = Location::span(file(), 10, 25, 12, 4).expect("cross-line span must construct");
    assert_eq!(parsed, expected);
}

#[test]
fn location_cross_line_span_round_trips_through_serde() {
    let span = Location::span(file(), 10, 25, 12, 4).expect("cross-line span must construct");
    let json = serde_json::to_string(&span).expect("serialize cross-line span");
    assert_eq!(json, CROSS_LINE_SPAN_JSON);
    let back: Location = serde_json::from_str(&json).expect("re-deserialize cross-line span");
    assert_eq!(span, back);
}

const SAME_LINE_INVERTED_SPAN_JSON: &str =
    r#"{"Span":{"file":"src/lib.rs","line_start":10,"col_start":25,"line_end":10,"col_end":4}}"#;

#[test]
fn location_rejects_same_line_inverted_span_on_wire() {
    let result: Result<Location, _> = serde_json::from_str(SAME_LINE_INVERTED_SPAN_JSON);
    assert!(
        result.is_err(),
        "same-line inverted columns must be rejected at the wire boundary, got: {result:?}"
    );
}

const INVERTED_LINE_SPAN_JSON: &str =
    r#"{"Span":{"file":"src/lib.rs","line_start":42,"col_start":5,"line_end":10,"col_end":5}}"#;

#[test]
fn location_rejects_inverted_line_span_on_wire() {
    let result: Result<Location, _> = serde_json::from_str(INVERTED_LINE_SPAN_JSON);
    assert!(
        result.is_err(),
        "line_end < line_start must be rejected at the wire boundary, got: {result:?}"
    );
}
