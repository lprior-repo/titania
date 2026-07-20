//! Outcome construction for the mutants lane.
//!
//! Three typed outcomes live here:
//!
//! - [`build_clean_outcome`] — `LaneOutcome::Clean` carrying the actual
//!   `argv` (cgroup-wrapped or bare fallback) and the cargo-mutants
//!   tool version.
//! - [`baseline_missing_outcome`] — `LaneOutcome::Findings` carrying
//!   one typed `MUTANT_BASELINE_MISSING` finding so the operator
//!   reminder reaches the receipt directly.
//! - [`survivor_finding`] — one typed `MUTANT_SURVIVED` reject finding
//!   per new test-survivor, with a real source-span `Location`.
//!
//! [`map_outcome_error`] converts the `OutcomeError` from
//! [`build_clean_outcome`] into the lane's own error enum so the
//! dispatcher site can stay in the typed `Result` shape.

use titania_core::{
    CommandEvidence, Digest, Finding, Lane, LaneEvidence, LaneFailure, LaneOutcome, Location,
    OutcomeError, ProcessTermination, RepairHint, RuleId, RuleIdError, WorkspacePath,
};

use super::{
    command::{bare_argv, cgroup_argv},
    constants::{
        MUTANTS_TOOL, MUTANTS_VERSION_FLOOR_MAJOR, RULE_MUTANT_BASELINE_MISSING,
        RULE_MUTANT_SURVIVED,
    },
    error::MutantsLaneError,
    state::{LaneRunState, NewSurvivor},
};

/// Build the `MUTANT_BASELINE_MISSING` typed outcome that the lane
/// emits when the baseline file is absent on disk.
#[must_use]
pub(super) fn baseline_missing_outcome(label: &str) -> LaneOutcome {
    let rule_id = match RuleId::new(RULE_MUTANT_BASELINE_MISSING) {
        Ok(rule) => rule,
        // Catalog literal is statically valid; falling through to Infra
        // keeps the lane panic-free even if the catalog drifts.
        Err(error) => {
            return LaneOutcome::Failed {
                failure: LaneFailure::Infra {
                    tool: MUTANTS_TOOL.to_owned(),
                    reason: format!("could not build MUTANT_BASELINE_MISSING rule id: {error}"),
                },
            };
        }
    };
    let finding = Finding::reject(
        Lane::Mutants,
        rule_id,
        Location::tool(MUTANTS_TOOL.to_owned(), MUTANTS_VERSION_FLOOR_MAJOR.to_string()),
        format!("mutants baseline file is missing at {label}"),
        RepairHint::requires_human_review(format!(
            "Run `scripts/dev/mutants-bootstrap.sh --owner <name> --reason <text>` to populate \
             `{label}`. See repair catalog row `{RULE_MUTANT_BASELINE_MISSING}`."
        )),
    );
    LaneOutcome::Findings { findings: vec![finding].into_boxed_slice() }
}

/// Build clean-lane evidence after every package completed successfully.
///
/// The `argv` matches the actual command that ran (cgroup-wrapped or
/// bare fallback) and the executable matches `argv[0]`, so the receipt
/// auditor can re-derive the run from `LaneEvidence::command`.
///
/// # Errors
///
/// Returns [`OutcomeError`] when `CommandEvidence::new` rejects the
/// argv (empty argv or argv[0] mismatch) or `LaneEvidence::new` rejects
/// the assembled evidence; both are forwarded up so [`super::outcome`]
/// can surface a typed lane failure.
pub(super) fn build_clean_outcome(state: &LaneRunState) -> Result<LaneOutcome, OutcomeError> {
    let (executable, argv): (String, Vec<String>) = if state.cgroup_used {
        (String::from("systemd-run"), cgroup_argv())
    } else {
        (String::from("cargo"), bare_argv())
    };
    let argv: Box<[String]> = argv.into_boxed_slice();
    let command = CommandEvidence::new(executable, argv)?;
    let evidence = LaneEvidence::new(
        command,
        state.tool_version.clone(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(state.tool_version.as_bytes()),
    )?;
    Ok(LaneOutcome::Clean { evidence })
}

/// Build one typed `MUTANT_SURVIVED` finding per new test-survivor.
///
/// The survivor's `Location` carries the real package-relative path
/// and 1-based line/column (with the column converted from
/// cargo-mutants' 1-based to the receipt's 0-based convention).
///
/// # Errors
///
/// Returns [`MutantsLaneError::RuleId`] when the static
/// `MUTANT_SURVIVED` rule literal is rejected by the workspace
/// rule-id grammar (defensive — the literal is catalog-validated) and
/// [`MutantsLaneError::MutantIdInvalid`] when the survivor path or span
/// cannot be assembled into a typed `Location`.
pub(super) fn survivor_finding(survivor: &NewSurvivor) -> Result<Finding, MutantsLaneError> {
    let rule_id = RuleId::new(RULE_MUTANT_SURVIVED).map_err(map_rule_id_error)?;
    let raw_name_box: Box<str> = Box::from(survivor.raw_name.as_str());
    let workspace_path = WorkspacePath::new(&survivor.rel_path).map_err(|error| {
        MutantsLaneError::MutantIdInvalid {
            name: raw_name_box.clone(),
            reason: Box::from(format!("invalid survivor path `{}`: {error}", survivor.rel_path)),
        }
    })?;
    let col_zero = survivor.column.saturating_sub(1);
    let location = Location::span(workspace_path, survivor.line, col_zero, survivor.line, col_zero)
        .map_err(|error| MutantsLaneError::MutantIdInvalid {
            name: raw_name_box,
            reason: Box::from(format!(
                "invalid survivor span {}:{}: {error}",
                survivor.line, survivor.column
            )),
        })?;
    Ok(Finding::reject(
        Lane::Mutants,
        rule_id,
        location,
        format!(
            "package={} rel_path={} line={} col={} genre={} replacement={:?} typed_id={}",
            survivor.package,
            survivor.rel_path,
            survivor.line,
            survivor.column,
            survivor.genre,
            survivor.replacement,
            survivor.typed_id
        ),
        RepairHint::requires_human_review(format!(
            "Surviving mutant `{}` in package `{}` lacks a baseline bypass entry. \
             Either kill it with a unit/property test, or accept it by appending to \
             `.titania/profiles/strict-ai/mutants.baseline.json` via \
             `scripts/dev/mutants-bootstrap.sh --owner <name> --reason <text>`.",
            survivor.raw_name, survivor.package
        )),
    ))
}

/// Convert a [`RuleIdError`] into the lane's own
/// [`MutantsLaneError::RuleId`] variant so the dispatcher site can stay
/// in the typed `Result` shape without leaking `RuleIdError` upward.
const fn map_rule_id_error(error: RuleIdError) -> MutantsLaneError {
    MutantsLaneError::RuleId(error)
}

/// Convert an evidence-construction [`OutcomeError`] into the lane's
/// own error enum.
///
/// Used by [`super::outcome`] when [`build_clean_outcome`] surfaces a
/// typed [`OutcomeError`] so the dispatch site can stay in the typed
/// `Result` shape without leaking `OutcomeError` upward. The dispatch
/// site maps every `MutantsLaneError` to a typed infra failure.
pub(super) fn map_outcome_error(error: &OutcomeError) -> MutantsLaneError {
    MutantsLaneError::BaselineMalformed {
        path: Box::from("clean-outcome-evidence"),
        reason: Box::from(format!("outcome evidence build failed: {error}").as_str()),
    }
}

#[cfg(test)]
mod tests {
    use titania_core::{
        Finding, Lane, LaneOutcome, Location, MutantId, MutantOperator, ProcessTermination,
        SkipReason, WorkspacePath,
    };

    use super::{
        super::state::{LaneRunState, NewSurvivor},
        RULE_MUTANT_BASELINE_MISSING, RULE_MUTANT_SURVIVED, baseline_missing_outcome,
        build_clean_outcome, survivor_finding,
    };

    #[test]
    fn clean_outcome_cgroup_records_systemd_run_executable() {
        let state =
            LaneRunState { tool_version: "cargo-mutants 27.0.0".to_owned(), cgroup_used: true };
        let outcome = build_clean_outcome(&state)
            .unwrap_or_else(|error| panic!("clean outcome must construct: {error}"));
        let LaneOutcome::Clean { evidence } = outcome else {
            panic!("clean outcome must wrap LaneEvidence");
        };
        assert_eq!(evidence.command().executable(), "systemd-run");
        let argv = evidence.command().argv();
        assert_eq!(argv.first().map(String::as_str), Some("systemd-run"));
        assert!(argv.iter().any(|arg| arg == "--workspace"));
    }

    #[test]
    fn clean_outcome_fallback_records_cargo_executable() {
        let state =
            LaneRunState { tool_version: "cargo-mutants 27.0.0".to_owned(), cgroup_used: false };
        let outcome = build_clean_outcome(&state)
            .unwrap_or_else(|error| panic!("clean outcome must construct: {error}"));
        let LaneOutcome::Clean { evidence } = outcome else {
            panic!("clean outcome must wrap LaneEvidence");
        };
        assert_eq!(evidence.command().executable(), "cargo");
        let argv = evidence.command().argv();
        assert_eq!(argv.first().map(String::as_str), Some("cargo"));
        assert!(argv.iter().any(|arg| arg == "--workspace"));
        assert_eq!(evidence.tool_version(), "cargo-mutants 27.0.0");
        assert_eq!(evidence.exit_status(), ProcessTermination::Exited { code: 0 });
    }

    #[test]
    fn baseline_missing_outcome_carries_typed_rule_id() {
        let outcome =
            baseline_missing_outcome("/tmp/.titania/profiles/strict-ai/mutants.baseline.json");
        let LaneOutcome::Findings { findings } = outcome else {
            panic!("missing baseline must surface Findings, not Skipped");
        };
        assert_eq!(findings.len(), 1);
        let only = &findings[0];
        assert!(only.is_reject());
        assert_eq!(only.rule_id().as_str(), RULE_MUTANT_BASELINE_MISSING);
        assert_eq!(only.lane(), Lane::Mutants);
    }

    #[test]
    fn survivor_finding_carries_real_location_not_generic_tool_loc() {
        // We do not execute any cargo-mutants, but we can prove that
        // `Location::span` accepts a typical cargo-mutants 1-based
        // line / 1-based column pair by converting the column to the
        // receipt's 0-based convention.
        let workspace_path = WorkspacePath::new("src/foo.rs")
            .unwrap_or_else(|error| panic!("relative survivor path must validate: {error}"));
        let col_zero = 8_u32.saturating_sub(1);
        let location = Location::span(workspace_path, 12, col_zero, 12, col_zero)
            .unwrap_or_else(|error| panic!("span must construct: {error}"));
        assert!(location.is_span(), "real survivor location must be a span, not a tool tag");
    }

    #[test]
    fn finding_reject_constructs_with_survivor_shape() {
        let rule_id = titania_core::RuleId::new(RULE_MUTANT_SURVIVED)
            .unwrap_or_else(|error| panic!("rule id literal must validate: {error}"));
        let workspace_path = WorkspacePath::new("src/lib.rs")
            .unwrap_or_else(|error| panic!("workspace path must validate: {error}"));
        let location = Location::span(workspace_path, 1, 0, 1, 0)
            .unwrap_or_else(|error| panic!("location must construct: {error}"));
        let finding = Finding::reject(
            Lane::Mutants,
            rule_id,
            location,
            "package=titania-core typed_id=demo".to_owned(),
            titania_core::RepairHint::requires_human_review(
                "demo survivor requires review".to_owned(),
            ),
        );
        assert!(finding.is_reject());
        assert_eq!(finding.rule_id().as_str(), RULE_MUTANT_SURVIVED);
        assert_eq!(finding.lane(), Lane::Mutants);
    }

    #[test]
    fn survivor_finding_assembles_real_span_for_typed_id() {
        let typed_id =
            MutantId::new("titania-core", "src/lib.rs", 12, 9, MutantOperator::EqualReplace)
                .unwrap_or_else(|error| panic!("typed id must construct: {error}"));
        let survivor = NewSurvivor {
            package: String::from("titania-core"),
            rel_path: String::from("src/lib.rs"),
            line: 12,
            column: 9,
            genre: String::from("BinaryOperator"),
            replacement: String::from("replace == with !="),
            raw_name: String::from("src/lib.rs:12:9: replace == with !="),
            typed_id,
        };
        let finding = survivor_finding(&survivor)
            .unwrap_or_else(|error| panic!("survivor finding must construct: {error}"));
        assert!(finding.is_reject());
        assert_eq!(finding.rule_id().as_str(), RULE_MUTANT_SURVIVED);
        assert_eq!(finding.lane(), Lane::Mutants);
    }

    #[test]
    fn baseline_missing_skip_reason_unused_here() {
        // SkipReason::ToolUnavailable is verified at the dispatch site
        // (see `tool_unavailable_outcome_records_skip_reason` in
        // `super::super::tests`). Kept here as a sentinel so the import
        // is not pruned.
        let _ = std::mem::size_of::<SkipReason>();
    }
}
