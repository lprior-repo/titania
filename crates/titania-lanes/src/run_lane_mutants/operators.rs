//! Cargo-mutants operator classification and per-mutation geometry.
//!
//! The lane keeps its own operator classifier because the v1.5 contract
//! fails *closed* on any `BinaryOperator` / `UnaryOperator` whose textual
//! name does not match a recognised pattern. The core
//! [`titania_core::MutantRecord::classify_operator`] is permissive by
//! construction (it defaults to `ArithmeticOpFlip`); the lane must
//! surface a typed [`MutantsLaneError::UnknownOperator`] instead so the
//! gate fails loud when cargo-mutants evolves past the closed set.

use titania_core::MutantOperator;

use super::error::MutantsLaneError;

/// Resolve a cargo-mutants record to its typed [`MutantOperator`] or
/// fail with [`MutantsLaneError::UnknownOperator`] when the genre ↔
/// name combination is outside the v1.5 closed set.
///
/// Recognised mappings (v1.5 spec §3 — closed [`MutantOperator`] set):
///
/// - `BinaryOperator` + `replace == with !=` → `EqualReplace`
/// - `BinaryOperator` + `replace != with ==` → `NotInserted`
/// - `BinaryOperator` + `replace && with ||` / `replace || with &&` →
///   `AndOr`
/// - `BinaryOperator` + arithmetic flip keyword (`+ ↔ -` or `* ↔ /`) →
///   `ArithmeticOpFlip`
/// - `UnaryOperator` + `delete ! in <name>` → `RemoveNegation`
/// - `FnValue` + integer literal replacement → `IntegerPlusOne` /
///   `IntegerMinusOne` when the cargo-mutants `replacement` field
///   carries the literal `1` or `-1`.
/// - `FnValue` + any other textual replacement → `DefaultReplace`.
/// - Everything else → [`MutantsLaneError::UnknownOperator`].
///
/// # Errors
///
/// Returns [`MutantsLaneError::UnknownOperator`] for recognised
/// `BinaryOperator` / `UnaryOperator` records whose textual name does
/// not match a v1.5 closed-set pattern, and for unknown genre tags that
/// do not fall into the `FnValue` default-replace bucket.
pub(super) fn operator_for_raw(
    genre: &str,
    name: &str,
    replacement: &str,
) -> Result<MutantOperator, MutantsLaneError> {
    match genre {
        "BinaryOperator" => binary_operator(name),
        "UnaryOperator" => unary_operator(name),
        "FnValue" => Ok(fn_value_operator(replacement)),
        _ => Err(MutantsLaneError::UnknownOperator {
            name: Box::from(name),
            genre: Box::from(genre),
            raw: Box::from(name),
        }),
    }
}

/// Classify a textual cargo-mutants `BinaryOperator` mutation name.
///
/// # Errors
///
/// Returns [`MutantsLaneError::UnknownOperator`] when the textual name
/// does not match a v1.5 closed-set binary pattern. The previous
/// coercion to [`MutantOperator::ArithmeticOpFlip`] is removed; an
/// unrecognised binary mutation now fails closed instead of silently
/// sticking a typed `ArithmeticOpFlip` id on the survivor.
fn binary_operator(name: &str) -> Result<MutantOperator, MutantsLaneError> {
    if name.contains("replace == with !=") {
        Ok(MutantOperator::EqualReplace)
    } else if name.contains("replace != with ==") {
        Ok(MutantOperator::NotInserted)
    } else if name.contains("replace && with ||") || name.contains("replace || with &&") {
        Ok(MutantOperator::AndOr)
    } else if name.contains("replace + with -")
        || name.contains("replace - with +")
        || name.contains("replace * with /")
        || name.contains("replace / with *")
    {
        Ok(MutantOperator::ArithmeticOpFlip)
    } else {
        Err(MutantsLaneError::UnknownOperator {
            name: Box::from(name),
            genre: Box::from("BinaryOperator"),
            raw: Box::from(name),
        })
    }
}

/// Classify a textual cargo-mutants `UnaryOperator` mutation name.
///
/// # Errors
///
/// Returns [`MutantsLaneError::UnknownOperator`] when the textual name
/// does not match a recognised `!`-removal pattern. We deliberately
/// keep this conservative — any new cargo-mutants unary shape must
/// trigger a contract amendment before the lane accepts it.
fn unary_operator(name: &str) -> Result<MutantOperator, MutantsLaneError> {
    if name.contains("delete !") {
        Ok(MutantOperator::RemoveNegation)
    } else {
        Err(MutantsLaneError::UnknownOperator {
            name: Box::from(name),
            genre: Box::from("UnaryOperator"),
            raw: Box::from(name),
        })
    }
}

/// Classify a cargo-mutants `FnValue` record by its textual
/// `replacement` value.
///
/// Integer literal replacements of `+1` / `-1` map to the dedicated
/// operators; everything else (default values such as `0`, `Default
/// ::default()`, `true`, `false`, …) collapses to
/// [`MutantOperator::DefaultReplace`]. We never error here because
/// `FnValue` surfacing is forward-compat by construction.
fn fn_value_operator(replacement: &str) -> MutantOperator {
    if replacement == "1" {
        MutantOperator::IntegerPlusOne
    } else if replacement == "-1" {
        MutantOperator::IntegerMinusOne
    } else {
        MutantOperator::DefaultReplace
    }
}

/// Convert a workspace-relative cargo-mutants file to a package-relative
/// path that [`titania_core::MutantId::new`] accepts.
///
/// # Errors
///
/// Returns [`MutantsLaneError::PathOutsidePackage`] when a
/// `crates/<other>/...` file is reported under a different package.
pub(super) fn relative_mutant_path<'a>(
    package: &str,
    file: &'a str,
) -> Result<&'a str, MutantsLaneError> {
    if let Some(workspace_path) = file.strip_prefix("crates/") {
        return workspace_path
            .strip_prefix(package)
            .and_then(|path| path.strip_prefix('/'))
            .ok_or_else(|| MutantsLaneError::PathOutsidePackage {
                name: Box::from(""),
                file: Box::from(file),
                package: Box::from(package),
            });
    }
    Ok(file.strip_prefix("./").map_or(file, |stripped| stripped))
}

#[cfg(test)]
mod tests {
    use titania_core::MutantOperator;

    use super::{
        binary_operator, fn_value_operator, operator_for_raw, relative_mutant_path, unary_operator,
    };

    #[test]
    fn binary_operator_classifies_known_patterns() {
        assert_eq!(
            binary_operator("replace == with != in foo")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::EqualReplace
        );
        assert_eq!(
            binary_operator("replace != with == in bar")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::NotInserted
        );
        assert_eq!(
            binary_operator("replace && with || in baz")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::AndOr
        );
        assert_eq!(
            binary_operator("replace || with && in qux")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::AndOr
        );
        assert_eq!(
            binary_operator("replace + with - in add")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::ArithmeticOpFlip
        );
        assert_eq!(
            binary_operator("replace * with / in mul")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::ArithmeticOpFlip
        );
    }

    #[test]
    fn binary_operator_unknown_pattern_fails_typed_infra() {
        let result = binary_operator("replace is_cargo_package_char -> bool with true");
        assert!(result.is_err(), "unknown binary pattern must surface typed error, not coerce");
    }

    #[test]
    fn binary_operator_does_not_coerce_unknown_to_arithmetic_op_flip() {
        // Prior implementations silently coerced anything that did
        // not match == / != / &&-|| into ArithmeticOpFlip. Verify
        // we never silently pick that branch by failing closed with
        // the typed `UnknownOperator` variant.
        let result = binary_operator("replace Foo with Bar");
        let Err(error) = result else {
            panic!("unknown binary pattern must fail closed");
        };
        let debug = format!("{error:?}");
        assert!(debug.contains("UnknownOperator"), "{debug}");
    }

    #[test]
    fn unary_operator_classifies_delete_negation() {
        assert_eq!(
            unary_operator("delete ! in is_foo")
                .unwrap_or_else(|error| panic!("known pattern must succeed: {error}")),
            MutantOperator::RemoveNegation
        );
    }

    #[test]
    fn unary_operator_unknown_fails_typed_infra() {
        assert!(unary_operator("delete == in foo").is_err());
    }

    #[test]
    fn fn_value_classifies_integer_literal_replacements() {
        assert_eq!(fn_value_operator("1"), MutantOperator::IntegerPlusOne);
        assert_eq!(fn_value_operator("-1"), MutantOperator::IntegerMinusOne);
        assert_eq!(fn_value_operator("0"), MutantOperator::DefaultReplace);
        assert_eq!(fn_value_operator("true"), MutantOperator::DefaultReplace);
        assert_eq!(fn_value_operator("Default::default()"), MutantOperator::DefaultReplace);
    }

    #[test]
    fn operator_for_raw_routes_genre_to_classifier() {
        assert_eq!(
            operator_for_raw("BinaryOperator", "replace == with !=", "true")
                .unwrap_or_else(|error| panic!("classified binary must succeed: {error}")),
            MutantOperator::EqualReplace
        );
        assert_eq!(
            operator_for_raw("UnaryOperator", "delete ! in flag", "true")
                .unwrap_or_else(|error| panic!("classified unary must succeed: {error}")),
            MutantOperator::RemoveNegation
        );
        assert_eq!(
            operator_for_raw("FnValue", "replace foo -> i32 with 1", "1")
                .unwrap_or_else(|error| panic!("classified FnValue must succeed: {error}")),
            MutantOperator::IntegerPlusOne
        );
    }

    #[test]
    fn operator_for_raw_unknown_genre_fails_typed_infra() {
        let result = operator_for_raw("MatchArmGuard", "replace guard", "true");
        assert!(result.is_err(), "unknown genre must surface typed error");
    }

    #[test]
    fn relative_mutant_path_strips_crates_prefix_for_matching_package() {
        let path = relative_mutant_path("titania-core", "crates/titania-core/src/lib.rs")
            .unwrap_or_else(|error| panic!("matching package prefix must succeed: {error}"));
        assert_eq!(path, "src/lib.rs");
    }

    #[test]
    fn relative_mutant_path_returns_bare_path_without_crates_prefix() {
        let path = relative_mutant_path("titania-core", "src/lib.rs")
            .unwrap_or_else(|error| panic!("bare path must succeed: {error}"));
        assert_eq!(path, "src/lib.rs");
    }

    #[test]
    fn relative_mutant_path_strips_dot_slash_prefix() {
        let path = relative_mutant_path("titania-core", "./src/lib.rs")
            .unwrap_or_else(|error| panic!("./ prefix must succeed: {error}"));
        assert_eq!(path, "src/lib.rs");
    }
}
