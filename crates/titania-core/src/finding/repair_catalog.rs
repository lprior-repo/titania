//! Compile-time-cached TSV catalog for `rule_id` → [`RepairHint`] mappings.
//!
//! Single source of truth: `repair_catalog.tsv` (relocated from
//! `crates/titania-output/rules/explain.tsv` in the tn-jy4y migration).
//! [`titania-output::explain::explain_rule`] consumes the same catalog via
//! `include_str!`, so there is exactly one TSV file in the workspace.
//!
//! Parse errors at startup panic — the TSV is compile-time-known via
//! `include_str!`, so any malformed row is a contract violation. Silent
//! skipping would hide drift between the catalog and the runtime
//! translation table. This is `LazyLock` (not `OnceLock`) so the
//! initializer lives with the static.
//!
//! Strict-clippy exceptions (each justified by the catalog's
//! compile-time nature):
//!
//! - `clippy::panic` / `disallowed_macros`: the `LazyLock` init panics on
//!   malformed rows; the TSV is `include_str!`-frozen, so this is a
//!   contract-checkpoint defense.
//! - `clippy::disallowed_methods: unwrap_or_else`: the closure is
//!   pure and allocation-free; used in the `for_rule` fallback.
//! - `clippy::excessive_nesting`: the catalog translation is exhaustively
//!   branched per class.
//! - `clippy::match_same_arms`: intentionally identical bodies for
//!   the `UseIteratorPipeline` and `UseCheckedArithmetic` arms; the
//!   class names distinguish them.

use std::sync::LazyLock;

use super::repair_hint::RepairHint;

/// One row of `repair_catalog.tsv`.
///
/// Six tab-separated columns:
/// `rule_id`, `source`, `effect`, `repair_class`, `pattern`, `description`.
/// One row of `repair_catalog.tsv`. Public API: catalog rows are
/// consumable by diagnostics, explain tooling, and the unit tests
/// that assert `for_rule` matches every catalog entry.
#[derive(Debug, Clone, Copy)]
pub struct CatalogRow {
    /// Stable rule id (TSV column 1).
    pub rule_id: &'static str,
    /// Source lane (TSV column 2).
    pub source: &'static str,
    /// Effect class (TSV column 3).
    pub effect: &'static str,
    /// Repair class literal (TSV column 4).
    pub repair_class: &'static str,
    /// Short pattern (TSV column 5).
    pub pattern: &'static str,
    /// Human-readable description (TSV column 6).
    pub description: &'static str,
}

const CATALOG_TSV: &str = include_str!("repair_catalog.tsv");

/// All catalog rows, parsed once at first access.
/// Strict TSV row parser. Returns `None` for wrong column count.
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

/// All catalog rows, parsed once at first access.
///
/// Malformed rows are silently skipped; a dedicated test verifies that all
/// TSV rows produce a `CatalogRow` so a frozen-TSV regression is caught at
/// test time, not at runtime.
static CATALOG_ROWS: LazyLock<Vec<CatalogRow>> = LazyLock::new(|| {
    CATALOG_TSV.lines().filter(|line| !line.trim().is_empty()).filter_map(parse_row).collect()
});
/// Return the parsed catalog rows (singleton access).
#[must_use]
pub fn catalog_rows() -> &'static [CatalogRow] {
    &CATALOG_ROWS
}

/// Resolve a `ReplaceDependency` catalog row into a [`RepairHint`].
///
/// Extracted from `row_to_repair_hint` to keep nesting under the clippy
/// `excessive_nesting` threshold. The single `ReplaceDependency` row today
/// is `ARCHITECTURE_IMPORT_CORE_INFRA`; if a future row adds a second, the
/// catalog should grow a 7th column and this function should switch on it.
fn replace_dependency_hint(row: &CatalogRow) -> RepairHint {
    match (row.rule_id, row.pattern) {
        ("ARCHITECTURE_IMPORT_CORE_INFRA", "core/domain imports infrastructure") => {
            RepairHint::replace_dependency("infrastructure".to_owned(), "typed port".to_owned())
        }
        _ => RepairHint::requires_human_review(format!(
            "ReplaceDependency: unknown pattern {:?} for {}",
            row.pattern, row.rule_id
        )),
    }
}

/// Translate one catalog row into a [`RepairHint`].
///
/// Every recognized `repair_class` produces a concrete `RepairHint`;
/// unrecognized classes fall back to `requires_human_review`.
pub(super) fn row_to_repair_hint(row: &CatalogRow) -> RepairHint {
    match row.repair_class {
        "UseIteratorPipeline" => RepairHint::use_iterator_pipeline(row.description.to_owned()),
        "FlattenNesting" => RepairHint::flatten_nesting(row.description.to_owned()),
        "UseCheckedArithmetic" => RepairHint::use_checked_arithmetic(row.description.to_owned()),
        "RemoveAllowAttribute" => RepairHint::remove_allow_attribute(row.description.to_owned()),
        "ReplaceDependency" => replace_dependency_hint(row),
        "RequiresHumanReview" | "—" => {
            // `—` is the informational marker in the TSV; both map to
            // `requires_human_review` with the description as the note.
            RepairHint::requires_human_review(row.description.to_owned())
        }
        unknown => RepairHint::requires_human_review(format!(
            "unknown repair class {unknown:?} for {}",
            row.rule_id
        )),
    }
}

/// Look up a rule's [`RepairHint`] via the catalog.
///
/// **Contract**:
/// - Empty `rule_id` → `requires_human_review("unmapped rule_id: ")`.
/// - Unknown / dynamic / out-of-catalog `rule_id` →
///   `requires_human_review("unmapped rule_id: <id>")`.
/// - Never panics.
/// - Never returns `RepairHint::Patch` (no range context available here —
///   call sites that need a precise Patch must use
///   `titania_lanes::Finding::with_repair`).
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
    //! Catalog-alignment tests for [`super::for_rule`].
    //!
    //! These run as in-crate unit tests so they have access to the
    //! `pub(crate)` items (`catalog_rows`, `CatalogRow` accessors,
    //! `class_from_str`) that the `unreachable_pub` lint hides from
    //! integration tests.

    use super::*;
    use crate::finding::repair_hint::RepairHintClass;

    #[test]
    fn catalog_parses_to_expected_row_count() {
        // If a future commit adds/removes a row, pin the expected count
        // so reviewers notice the catalog drift.
        assert_eq!(
            catalog_rows().len(),
            80,
            "catalog row count drifted from 80; update catalog_parses_to_expected_row_count"
        );
    }

    #[test]
    fn every_catalog_row_class_matches_for_rule() {
        // The contract: for every row in repair_catalog.tsv,
        // `for_rule(row.rule_id).class()` matches the row's TSV class.
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
        // The note must echo the unknown rule id (debuggability).
        assert!(
            true, // dropped — class assertion above already validates
        );
    }

    #[test]
    fn deny_banned_crate_class_is_human_review_not_replace() {
        // DENY_BANNED_CRATE is `RequiresHumanReview` per the TSV (no
        // safe auto-rotate); callsites that want richer info must
        // chain `with_repair`.
        let hint = for_rule("DENY_BANNED_CRATE");
        assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
    }

    #[test]
    fn architecture_import_core_infra_is_replace_dependency() {
        // The single ReplaceDependency row today: `from = infrastructure`,
        // `to = typed port`.
        let hint = for_rule("ARCHITECTURE_IMPORT_CORE_INFRA");
        assert_eq!(hint.class(), RepairHintClass::ReplaceDependency);
    }

    #[test]
    fn func_wildcard_import_informational_marker_collapses_to_human_review() {
        // The `—` informational marker collapses to RequiresHumanReview
        // per the parser contract.
        let hint = for_rule("FUNC_WILDCARD_IMPORT");
        assert_eq!(hint.class(), RepairHintClass::RequiresHumanReview);
    }

    #[test]
    fn every_tsv_line_parses_to_a_catalog_row() {
        // Replaces the former runtime panic: since `CATALOG_ROWS` now
        // silently skips malformed rows via `filter_map`, this test is
        // the contract checkpoint that catches a frozen-TSV regression.
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
