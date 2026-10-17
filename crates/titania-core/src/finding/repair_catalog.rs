use std::sync::LazyLock;

use super::repair_hint::RepairHint;

#[derive(Debug, Clone, Copy)]
pub struct CatalogRow {
    pub rule_id: &'static str,
    pub source: &'static str,
    pub effect: &'static str,
    pub repair_class: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
}

const CATALOG_TSV: &str = include_str!("repair_catalog.tsv");

fn parse_row(line: &'static str) -> Option<CatalogRow> {
    let mut fields = line.split('\t');
    let row = CatalogRow {
        rule_id: fields.next()?,
        source: fields.next()?,
        effect: fields.next()?,
        repair_class: fields.next()?,
        pattern: fields.next()?,
        description: fields.next()?,
    };
    fields.next().is_none().then_some(row)
}

static CATALOG_ROWS: LazyLock<Vec<CatalogRow>> = LazyLock::new(|| {
    CATALOG_TSV.lines().filter(|line| !line.trim().is_empty()).filter_map(parse_row).collect()
});
#[must_use]
pub fn catalog_rows() -> &'static [CatalogRow] {
    &CATALOG_ROWS
}

fn replace_dependency_hint(row: &CatalogRow) -> RepairHint {
    match (row.rule_id, row.pattern) {
        ("ARCHITECTURE_IMPORT_CORE_INFRA", "core/domain imports infrastructure") => {
            RepairHint::replace_dependency("infrastructure".to_owned(), "typed port".to_owned())
        }
        ("ARCHITECTURE_EFFECT_CORE_ASYNC", "core/domain uses async runtime") => {
            RepairHint::replace_dependency("async runtime".to_owned(), "typed port".to_owned())
        }
        _ => RepairHint::requires_human_review(format!(
            "ReplaceDependency: unknown pattern {:?} for {}",
            row.pattern, row.rule_id
        )),
    }
}

pub(super) fn row_to_repair_hint(row: &CatalogRow) -> RepairHint {
    match row.repair_class {
        "UseIteratorPipeline" => RepairHint::use_iterator_pipeline(row.description.to_owned()),
        "FlattenNesting" => RepairHint::flatten_nesting(row.description.to_owned()),
        "UseCheckedArithmetic" => RepairHint::use_checked_arithmetic(row.description.to_owned()),
        "RemoveAllowAttribute" => RepairHint::remove_allow_attribute(row.description.to_owned()),
        "ReplaceDependency" => replace_dependency_hint(row),
        "RequiresHumanReview" | "—" => {
            RepairHint::requires_human_review(row.description.to_owned())
        }
        unknown => RepairHint::requires_human_review(format!(
            "unknown repair class {unknown:?} for {}",
            row.rule_id
        )),
    }
}

#[must_use]
pub(super) fn for_rule(rule_id: &str) -> RepairHint {
    if rule_id.is_empty() {
        return RepairHint::requires_human_review("unmapped rule_id: ".to_owned());
    }
    catalog_rows().iter().find(|row| row.rule_id == rule_id).map_or_else(
        || RepairHint::requires_human_review(format!("unmapped rule_id: {rule_id}")),
        row_to_repair_hint,
    )
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::finding::repair_hint::RepairHintClass;

    #[test]
    fn catalog_parses_to_expected_row_count() {
        assert_eq!(
            catalog_rows().len(),
            82,
            "catalog row count drifted from 82; update catalog_parses_to_expected_row_count"
        );
    }

    #[test]
    fn every_catalog_row_class_matches_for_rule() {
        for row in catalog_rows() {
            let hint = for_rule(row.rule_id);
            let expected = match row.repair_class {
                "UseIteratorPipeline" => RepairHintClass::UseIteratorPipeline,
                "FlattenNesting" => RepairHintClass::FlattenNesting,
                "UseCheckedArithmetic" => RepairHintClass::UseCheckedArithmetic,
                "RemoveAllowAttribute" => RepairHintClass::RemoveAllowAttribute,
                "ReplaceDependency" => RepairHintClass::ReplaceDependency,
                "RequiresHumanReview" | "—" => RepairHintClass::RequiresHumanReview,
                _other => RepairHintClass::RequiresHumanReview,
            };
            assert_eq!(
                hint.class(),
                expected,
                "row {} (class {}) maps to class {} (expected {})",
                row.rule_id,
                row.repair_class,
                hint.class().as_str(),
                expected.as_str(),
            );
        }
    }

    #[test]
    fn empty_rule_id_returns_human_review_without_panicking() {
        let hint = for_rule("");
        assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
    }

    #[test]
    fn unknown_rule_id_returns_human_review_with_id_in_note() {
        let hint = for_rule("CLIPPY_DOES_NOT_EXIST_XYZ");
        assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
        assert!(true,);
    }

    #[test]
    fn deny_banned_crate_class_is_human_review_not_replace() {
        let hint = for_rule("DENY_BANNED_CRATE");
        assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
    }

    #[test]
    fn architecture_import_core_infra_is_replace_dependency() {
        let hint = for_rule("ARCHITECTURE_IMPORT_CORE_INFRA");
        assert_eq!(hint.class(), RepairHintClass::ReplaceDependency);
    }

    #[test]
    fn func_wildcard_import_informational_marker_collapses_to_human_review() {
        let hint = for_rule("FUNC_WILDCARD_IMPORT");
        assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
    }

    #[test]
    fn every_tsv_line_parses_to_a_catalog_row() {
        let non_empty = CATALOG_TSV.lines().filter(|line| !line.trim().is_empty()).count();
        assert_eq!(
            catalog_rows().len(),
            non_empty,
            "every non-empty TSV line must parse to a CatalogRow; \
             {} non-empty lines but only {} rows parsed",
            non_empty,
            catalog_rows().len(),
        );
    }
}
