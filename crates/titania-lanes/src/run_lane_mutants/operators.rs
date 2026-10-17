use titania_core::MutantOperator;

use super::error::MutantsLaneError;

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

const BINARY_OPERATOR_PATTERNS: &[(&str, MutantOperator)] = &[
    ("replace == with !=", MutantOperator::EqualReplace),
    ("replace != with ==", MutantOperator::NotInserted),
    ("replace && with ||", MutantOperator::AndOr),
    ("replace || with &&", MutantOperator::AndOr),
    ("replace + with -", MutantOperator::ArithmeticOpFlip),
    ("replace - with +", MutantOperator::ArithmeticOpFlip),
    ("replace * with /", MutantOperator::ArithmeticOpFlip),
    ("replace / with *", MutantOperator::ArithmeticOpFlip),
];

fn binary_operator(name: &str) -> Result<MutantOperator, MutantsLaneError> {
    BINARY_OPERATOR_PATTERNS.iter().find(|(pattern, _)| name.contains(pattern)).map_or_else(
        || {
            Err(MutantsLaneError::UnknownOperator {
                name: Box::from(name),
                genre: Box::from("BinaryOperator"),
                raw: Box::from(name),
            })
        },
        |(_, operator)| Ok(*operator),
    )
}

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

fn fn_value_operator(replacement: &str) -> MutantOperator {
    if replacement == "1" {
        MutantOperator::IntegerPlusOne
    } else if replacement == "-1" {
        MutantOperator::IntegerMinusOne
    } else {
        MutantOperator::DefaultReplace
    }
}

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
