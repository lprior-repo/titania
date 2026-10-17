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

#[test]
fn func_loops_for_skipped_inside_cfg_test_module() -> TestResult {
    let outcome = run_fixture("allowed_unbounded_loop_in_cfg_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    assert!(
        !ids.iter().any(|id| id == "BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP must not fire inside #[cfg(test)] mod tests, got: {ids:?}",
    );
    assert!(
        !ids.iter().any(|id| id.contains("BOUNDED_EXECUTION")),
        "No BOUNDED_EXECUTION_* rule may fire inside #[cfg(test)] mod tests, got: {ids:?}",
    );
    Ok(())
}

#[test]
fn func_loops_for_fires_in_prod_skips_in_cfg_test_module() -> TestResult {
    let outcome = run_fixture("real_unbounded_loop_in_prod_and_cfg_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    let loop_count = ids.iter().filter(|id| *id == "BOUNDED_EXECUTION_UNBOUNDED_LOOP").count();
    assert_eq!(
        loop_count, 1,
        "exactly one BOUNDED_EXECUTION_UNBOUNDED_LOOP finding (production code), got {ids:?}",
    );
    Ok(())
}

#[test]
fn func_loops_for_skipped_inside_cfg_all_test_module() -> TestResult {
    let outcome = run_fixture("allowed_unbounded_loop_in_cfg_all_test_module.rs")?;
    let ids = finding_rule_ids(&outcome);
    assert!(
        !ids.iter().any(|id| id == "BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP must not fire inside #[cfg(all(test, ...))] mod tests, got: {ids:?}",
    );
    Ok(())
}

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
