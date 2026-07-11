//! Contract tests for the `#[cfg(test)]` module skipping in FUNC_* rules.
//!
//! The ast-grep FUNC_* rules (loops, prints, unwrap_or, result_string,
//! nesting, recursion) must skip matches inside `#[cfg(test)]` inline
//! modules in `src/` files per v1-spec §9.10 (source/test split). The
//! BYPASS_* rules do NOT skip test modules — `#[allow]` inside a test
//! module is still a bypass.
//!
//! Bead: tn-k0rw

use std::{
    error::Error,
    path::{Path, PathBuf},
};

use titania_core::LaneOutcome;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_root(name: &str) -> PathBuf {
    let base = env!("CARGO_MANIFEST_DIR");
    Path::new(base).join("tests").join("fixtures").join("ast_grep").join("functional").join(name)
}

fn rules_yaml() -> [&'static str; 3] {
    [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ]
}

fn run_fixture(name: &str) -> Result<LaneOutcome, Box<dyn Error>> {
    let paths = vec![fixture_root(name)];
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml(), &paths, &[])?;
    Ok(outcome)
}

fn finding_rule_ids(outcome: &LaneOutcome) -> Vec<String> {
    match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        _ => Vec::new(),
    }
}

// ===========================================================================
// FUNC_LOOPS_FOR must skip #[cfg(test)] modules
// ===========================================================================

/// A `for` loop inside `#[cfg(test)] mod tests` produces NO findings.
#[test]
fn func_loops_for_skipped_inside_cfg_test_module() -> TestResult {
    let outcome = run_fixture("allowed_for_in_cfg_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    assert!(
        !ids.iter().any(|id| id == "FUNC_LOOPS_FOR"),
        "FUNC_LOOPS_FOR must not fire inside #[cfg(test)] mod tests, got: {ids:?}",
    );
    assert!(
        !ids.iter().any(|id| id.contains("FUNC_LOOPS")),
        "No FUNC_LOOPS_* rule may fire inside #[cfg(test)] mod tests, got: {ids:?}",
    );
    Ok(())
}

/// A `for` loop in production code alongside a `for` loop in `#[cfg(test)] mod tests`
/// produces a FUNC_LOOPS_FOR finding for the production code only.
#[test]
fn func_loops_for_fires_in_prod_skips_in_cfg_test_module() -> TestResult {
    let outcome = run_fixture("real_for_in_prod_and_cfg_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    let for_count = ids.iter().filter(|id| *id == "FUNC_LOOPS_FOR").count();
    assert_eq!(for_count, 1, "exactly one FUNC_LOOPS_FOR finding (production code), got {ids:?}",);
    Ok(())
}

/// A `for` loop inside `#[cfg(all(test, feature = "debug"))] mod tests` is skipped.
#[test]
fn func_loops_for_skipped_inside_cfg_all_test_module() -> TestResult {
    let outcome = run_fixture("allowed_for_in_cfg_all_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    assert!(
        !ids.iter().any(|id| id == "FUNC_LOOPS_FOR"),
        "FUNC_LOOPS_FOR must not fire inside #[cfg(all(test, ...))] mod tests, got: {ids:?}",
    );
    Ok(())
}

// ===========================================================================
// FUNC_PRINT_STDOUT must skip #[cfg(test)] modules
// ===========================================================================

/// `println!` inside `#[cfg(test)] mod tests` produces NO FUNC_PRINT_STDOUT finding.
#[test]
fn func_print_stdout_skipped_inside_cfg_test_module() -> TestResult {
    let outcome = run_fixture("allowed_println_in_cfg_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    assert!(
        !ids.iter().any(|id| id == "FUNC_PRINT_STDOUT"),
        "FUNC_PRINT_STDOUT must not fire inside #[cfg(test)] mod tests, got: {ids:?}",
    );
    Ok(())
}
