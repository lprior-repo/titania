//! v1.5 contract tests for `MutantId` invariants and parsing.
//!
//! Covers:
//! - canonical string construction and round-trip with `parse`,
//! - bounded package / path validation (NUL, control, backslash, embedded
//!   `:`, drive prefix, UNC form, `..`, length cap),
//! - 1-based line / column enforcement with `u32::MAX` boundary,
//! - all 8 [`MutantOperator`] variants as exact serde wire forms,
//! - typed [`MutantOperatorError`] from [`MutantOperator::from_str`].

use titania_core::{
    MUTANT_PATH_MAX_LEN, MUTANT_PKG_MAX_LEN, MutantId, MutantIdError, MutantOperator,
    MutantOperatorError, PathSegmentError,
};

#[test]
fn constructs_canonical_string() {
    let id =
        MutantId::new("titania-core", "src/lane.rs", 42, 13, MutantOperator::EqualReplace).unwrap();
    assert_eq!(id.as_str(), "titania-core::src/lane.rs:42:13:equal_replace");
}

#[test]
fn rejects_empty_package() {
    let result = MutantId::new("", "src/lane.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::EmptyPackage);
}

#[test]
fn rejects_empty_path() {
    let result = MutantId::new("pkg", "", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::EmptyPath);
}

#[test]
fn rejects_zero_line() {
    let result = MutantId::new("pkg", "src/lane.rs", 0, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::LineNotPositive);
}

#[test]
fn rejects_zero_column() {
    let result = MutantId::new("pkg", "src/lane.rs", 1, 0, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::ColNotPositive);
}

#[test]
fn rejects_absolute_path() {
    let result = MutantId::new("pkg", "/abs/path.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PathAbsolute);
}

#[test]
fn package_returns_prefix() {
    let id =
        MutantId::new("titania-core", "src/lane.rs", 1, 1, MutantOperator::NotInserted).unwrap();
    assert_eq!(id.package(), "titania-core");
}

#[test]
fn location_returns_suffix() {
    let id =
        MutantId::new("titania-core", "src/lane.rs", 1, 1, MutantOperator::NotInserted).unwrap();
    assert_eq!(id.location(), "src/lane.rs:1:1:not_inserted");
}

#[test]
fn accepts_u32_max_line_and_col_via_new() {
    let id =
        MutantId::new("pkg", "src/a.rs", u32::MAX, u32::MAX, MutantOperator::EqualReplace).unwrap();
    assert_eq!(id.as_str(), "pkg::src/a.rs:4294967295:4294967295:equal_replace");
}

#[test]
fn accepts_u32_max_line_via_parse() {
    let raw = "pkg::src/a.rs:4294967295:7:equal_replace";
    let id = MutantId::parse(raw).unwrap();
    assert_eq!(id.as_str(), raw);
}

#[test]
fn accepts_u32_max_column_via_parse() {
    let raw = "pkg::src/a.rs:7:4294967295:equal_replace";
    let id = MutantId::parse(raw).unwrap();
    assert_eq!(id.as_str(), raw);
}

#[test]
fn parse_rejects_line_overflow() {
    // 4294967296 == u32::MAX + 1; checked arithmetic in parse_u32 returns None.
    let err = MutantId::parse("pkg::src/a.rs:4294967296:1:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::LineNotAnInteger);
}

#[test]
fn parse_rejects_column_overflow() {
    let err = MutantId::parse("pkg::src/a.rs:1:4294967296:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::ColNotAnInteger);
}

#[test]
fn parse_accepts_long_path_within_cap() {
    let padded = format!("src/{}/a.rs", "deep/".repeat(10));
    assert!(padded.len() <= MUTANT_PATH_MAX_LEN);
    let raw = format!("pkg::{padded}:1:1:equal_replace");
    let id = MutantId::parse(&raw).unwrap();
    assert_eq!(id.as_str(), raw);
}

#[test]
fn parse_round_trips_with_new() {
    let canonical =
        MutantId::new("titania-core", "src/lane.rs", 42, 13, MutantOperator::EqualReplace).unwrap();
    let parsed = MutantId::parse(canonical.as_str()).unwrap();
    assert_eq!(parsed, canonical);
}

#[test]
fn parse_accepts_long_path() {
    let raw = "pkg::src/deep/nested/file.rs:123:7:default_replace";
    let parsed = MutantId::parse(raw).unwrap();
    assert_eq!(parsed.as_str(), raw);
}

#[test]
fn parse_rejects_missing_pkg_separator() {
    let err = MutantId::parse("no_separator").unwrap_err();
    assert_eq!(err, MutantIdError::MissingSeparator("no_separator".to_owned()));
}

#[test]
fn parse_rejects_missing_operator() {
    // No `:` separator at all after `<pkg>::`, so the operator suffix is
    // genuinely absent (we need three `:` to fix operator / col / line /
    // path).
    let err = MutantId::parse("pkg::path").unwrap_err();
    assert!(matches!(err, MutantIdError::MissingOperator(_)));
}

#[test]
fn parse_rejects_too_few_colons() {
    // Only one `:` in `rest` is short of the required three positional
    // separators, so the rsplit_once cascade can place the operator
    // (`equal_replace`, a real closed-set value) but `before_op` has no
    // further `:` to split on for the col value — rejected as
    // `MissingOperator`.
    let err = MutantId::parse("pkg::path:equal_replace").unwrap_err();
    assert!(matches!(err, MutantIdError::MissingOperator(_)), "got {err:?}");
}

#[test]
fn parse_rejects_too_many_colons() {
    // Four `:` in `rest` is excessive; under the canonical positional
    // parser the leftmost `:` must belong to the path, so the form is
    // ambiguous and rejected as `PathContainsColon` rather than as
    // `MissingOperator`.
    let err = MutantId::parse("pkg::path:1:2:3:equal_replace").unwrap_err();
    assert!(matches!(err, MutantIdError::PathContainsColon(_)), "got {err:?}");
}

#[test]
fn parse_rejects_colon_in_path() {
    // `pkg::src/foo:bar.rs:1:1:equal_replace` carries an extra `:` in the
    // path segment. Under the canonical right-edge parser the four
    // rightmost colons are line / col / (line replacement colons) /
    // operator, so the recovered path contains `:` and the form is
    // rejected outright.
    let err = MutantId::parse("pkg::src/foo:bar.rs:1:1:equal_replace").unwrap_err();
    let s = format!("{err:?}");
    assert!(matches!(err, MutantIdError::PathContainsColon(_)), "got {s}");
}

#[test]
fn parse_rejects_unknown_operator() {
    let err = MutantId::parse("pkg::path:1:1:not_a_real_op").unwrap_err();
    assert_eq!(err, MutantIdError::UnknownOperator("not_a_real_op".to_owned()));
}

#[test]
fn parse_rejects_non_numeric_line() {
    let err = MutantId::parse("pkg::path:line_one:1:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::LineNotAnInteger);
}

#[test]
fn parse_rejects_non_numeric_column() {
    let err = MutantId::parse("pkg::path:1:bad:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::ColNotAnInteger);
}

#[test]
fn parse_rejects_zero_line() {
    let err = MutantId::parse("pkg::path:0:1:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::LineNotPositive);
}

#[test]
fn parse_rejects_zero_column() {
    let err = MutantId::parse("pkg::path:1:0:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::ColNotPositive);
}

#[test]
fn parse_rejects_empty_package() {
    let err = MutantId::parse("::path:1:1:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::EmptyPackage);
}

#[test]
fn parse_rejects_empty_path() {
    let err = MutantId::parse("pkg:::1:1:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::EmptyPath);
}

#[test]
fn parse_rejects_absolute_path() {
    let err = MutantId::parse("pkg::/abs:1:1:equal_replace").unwrap_err();
    assert_eq!(err, MutantIdError::PathAbsolute);
}

// ---- Bounded package / path validation (NUL / control / backslash /
//      absolute / colon policy) ----------------------------------------

#[test]
fn new_rejects_package_with_nul_byte() {
    let result = MutantId::new("pkg\0", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PackageInvalid(PathSegmentError::ContainsNull));
}

#[test]
fn new_rejects_path_with_nul_byte() {
    let result = MutantId::new("pkg", "src/a\0.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PathInvalid(PathSegmentError::ContainsNull));
}

#[test]
fn new_rejects_package_with_control_byte() {
    let result = MutantId::new("pkg\tfoo", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PackageInvalid(PathSegmentError::ControlByte(0x09))
    );
}

#[test]
fn new_rejects_path_with_control_byte() {
    let result = MutantId::new("pkg", "src/a\x07.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PathInvalid(PathSegmentError::ControlByte(0x07))
    );
}

#[test]
fn new_rejects_package_with_backslash() {
    let result = MutantId::new("pkg\\foo", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PackageInvalid(PathSegmentError::ContainsBackslash)
    );
}

#[test]
fn new_rejects_path_with_backslash() {
    let result = MutantId::new("pkg", "src\\a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PathInvalid(PathSegmentError::ContainsBackslash)
    );
}

#[test]
fn new_rejects_package_with_embedded_colon() {
    let result = MutantId::new("my:pkg", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PackageInvalid(PathSegmentError::ContainsColon));
}

#[test]
fn new_rejects_package_drive_prefix() {
    let result = MutantId::new("C:foo", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PackageInvalid(PathSegmentError::DriveAbsolute("C:".to_owned()))
    );
}

#[test]
fn new_rejects_path_drive_prefix() {
    let result = MutantId::new("pkg", "C:foo.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PathInvalid(PathSegmentError::DriveAbsolute("C:".to_owned()))
    );
}

#[test]
fn new_rejects_package_unc_backslash() {
    let result = MutantId::new("\\\\srv", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PackageInvalid(PathSegmentError::UncForm));
}

#[test]
fn new_rejects_package_unc_forward_slash() {
    let result = MutantId::new("//srv/share", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PackageInvalid(PathSegmentError::UncForm));
}

#[test]
fn new_rejects_package_dot_dot_component() {
    let result = MutantId::new("..", "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PackageInvalid(PathSegmentError::ContainsDotDot)
    );
}

#[test]
fn new_rejects_path_dot_dot_component() {
    let result = MutantId::new("pkg", "src/../a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PathInvalid(PathSegmentError::ContainsDotDot));
}

#[test]
fn new_rejects_path_trailing_dot_dot() {
    let result = MutantId::new("pkg", "src/foo/..", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(result.unwrap_err(), MutantIdError::PathInvalid(PathSegmentError::ContainsDotDot));
}

#[test]
fn new_rejects_package_too_long() {
    let over = "a".repeat(MUTANT_PKG_MAX_LEN + 1);
    let result = MutantId::new(&over, "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PackageTooLong { found: over.len(), max: MUTANT_PKG_MAX_LEN }
    );
}

#[test]
fn new_accepts_package_at_max_len() {
    let at_cap = "a".repeat(MUTANT_PKG_MAX_LEN);
    let result = MutantId::new(&at_cap, "src/a.rs", 1, 1, MutantOperator::EqualReplace);
    assert!(result.is_ok(), "got {result:?}");
}

#[test]
fn new_rejects_path_too_long() {
    let over = "a".repeat(MUTANT_PATH_MAX_LEN + 1);
    let result = MutantId::new("pkg", &over, 1, 1, MutantOperator::EqualReplace);
    assert_eq!(
        result.unwrap_err(),
        MutantIdError::PathTooLong { found: over.len(), max: MUTANT_PATH_MAX_LEN }
    );
}

#[test]
fn new_accepts_path_at_max_len() {
    let at_cap = "a".repeat(MUTANT_PATH_MAX_LEN);
    let result = MutantId::new("pkg", &at_cap, 1, 1, MutantOperator::EqualReplace);
    assert!(result.is_ok(), "got {result:?}");
}

// ---- All 8 operator round-trips and exact serde wire forms ------------

const ALL_OPERATORS: &[(MutantOperator, &str)] = &[
    (MutantOperator::EqualReplace, "equal_replace"),
    (MutantOperator::NotInserted, "not_inserted"),
    (MutantOperator::AndOr, "and_or"),
    (MutantOperator::IntegerPlusOne, "integer_plus_one"),
    (MutantOperator::IntegerMinusOne, "integer_minus_one"),
    (MutantOperator::ArithmeticOpFlip, "arithmetic_op_flip"),
    (MutantOperator::DefaultReplace, "default_replace"),
    (MutantOperator::RemoveNegation, "remove_negation"),
];

#[test]
fn operator_as_str_matches_closed_set() {
    assert_eq!(ALL_OPERATORS.len(), 8);
    for (op, name) in ALL_OPERATORS {
        assert_eq!(op.as_str(), *name, "as_str mismatch for {op:?}");
    }
}

#[test]
fn operator_from_str_round_trips_each_variant() {
    for (op, name) in ALL_OPERATORS {
        let parsed: MutantOperator = name.parse().unwrap_or_else(|_| panic!("parse {name}"));
        assert_eq!(parsed, *op, "parse mismatch for {name}");
    }
}

#[test]
fn operator_from_str_rejects_unknown_with_typed_error() {
    let err = "not_a_real_op".parse::<MutantOperator>().unwrap_err();
    assert_eq!(err, MutantOperatorError::Unknown("not_a_real_op".to_owned()));
}

#[test]
fn operator_serde_serializes_to_exact_wire_form() {
    for (op, name) in ALL_OPERATORS {
        let json = serde_json::to_string(op).unwrap();
        assert_eq!(json, format!("\"{name}\""), "serialization mismatch for {op:?}");
    }
}

#[test]
fn operator_serde_deserializes_from_exact_wire_form() {
    for (op, name) in ALL_OPERATORS {
        let json = format!("\"{name}\"");
        let parsed: MutantOperator = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, *op, "deserialization mismatch for {name}");
    }
}

#[test]
fn operator_serde_rejects_unknown_wire_form() {
    let err = serde_json::from_str::<MutantOperator>("\"bogus_op\"").unwrap_err();
    let rendered = err.to_string();
    assert!(rendered.contains("bogus_op"), "diagnostic missing literal: {rendered}");
}

#[test]
fn new_emits_exact_canonical_wire_for_each_operator() {
    for (op, name) in ALL_OPERATORS {
        let id = MutantId::new("pkg", "src/a.rs", 1, 1, *op).unwrap();
        let expected = format!("pkg::src/a.rs:1:1:{name}");
        assert_eq!(id.as_str(), expected, "wire mismatch for {op:?}");
    }
}

#[test]
fn parse_round_trips_each_operator() {
    for (op, name) in ALL_OPERATORS {
        let raw = format!("pkg::src/a.rs:1:1:{name}");
        let id = MutantId::parse(&raw).unwrap();
        let canonical = MutantId::new("pkg", "src/a.rs", 1, 1, *op).unwrap();
        assert_eq!(id, canonical, "round-trip mismatch for {op:?}");
    }
}

#[test]
fn serde_deserializes_through_parse() {
    let json = "\"pkg::src/file.rs:1:1:equal_replace\"";
    let id: MutantId = serde_json::from_str(json).unwrap();
    assert_eq!(id.as_str(), "pkg::src/file.rs:1:1:equal_replace");
}

#[test]
fn serde_deserialize_rejects_malformed_string() {
    let json = "\"pkg::src/file.rs:1:1:not_a_real_op\"";
    let err = serde_json::from_str::<MutantId>(json).unwrap_err();
    assert!(err.to_string().contains("not_a_real_op"));
}

#[test]
fn from_str_parses_canonical_form() {
    let id: MutantId = "pkg::src/file.rs:1:1:equal_replace".parse().unwrap();
    assert_eq!(id.as_str(), "pkg::src/file.rs:1:1:equal_replace");
}
