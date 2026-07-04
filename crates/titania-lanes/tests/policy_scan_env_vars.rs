//! Behavior tests for the environment-variable policy scanner.

use std::collections::BTreeMap;

use titania_lanes::{LaneReport, RuleIdError, policy_scan::env_vars::scan_env_vars_with};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn scan_with(map: &BTreeMap<&str, Option<&str>>) -> Result<LaneReport, RuleIdError> {
    let owned = map
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.map(ToOwned::to_owned)))
        .collect::<BTreeMap<String, Option<String>>>();
    let env = move |name: &str| owned.get(name).cloned().flatten();
    let mut report = LaneReport::new();
    scan_env_vars_with(&mut report, &env)?;
    Ok(report)
}

fn only<'a>(name: &'a str, value: &'a str) -> BTreeMap<&'a str, Option<&'a str>> {
    BTreeMap::from([(name, Some(value))])
}

fn all_set(value: &str) -> BTreeMap<&str, Option<&str>> {
    BTreeMap::from([
        ("RUSTFLAGS", Some(value)),
        ("CARGO_ENCODED_RUSTFLAGS", Some(value)),
        ("RUSTC_WRAPPER", Some(value)),
        ("RUSTC_WORKSPACE_WRAPPER", Some(value)),
        ("RUSTC_BOOTSTRAP", Some(value)),
    ])
}

fn all_unset() -> BTreeMap<&'static str, Option<&'static str>> {
    BTreeMap::from([
        ("RUSTFLAGS", None),
        ("CARGO_ENCODED_RUSTFLAGS", None),
        ("RUSTC_WRAPPER", None),
        ("RUSTC_WORKSPACE_WRAPPER", None),
        ("RUSTC_BOOTSTRAP", None),
    ])
}

fn finding_for<'a>(report: &'a LaneReport, rule: &str) -> &'a titania_lanes::Finding {
    report
        .findings()
        .iter()
        .find(|finding| finding.rule().as_str() == rule)
        .expect("expected finding rule")
}

#[test]
fn all_forbidden_vars_emit_distinct_findings() -> TestResult {
    let report = scan_with(&all_set("some_value"))?;
    assert_eq!(report.finding_count(), 5);
    assert_eq!(finding_for(&report, "BYPASS_ENV_RUSTFLAGS").path(), "env");
    assert_eq!(finding_for(&report, "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS").line(), 0);
    assert!(
        finding_for(&report, "BYPASS_ENV_RUSTC_WRAPPER").message().contains("bypass lane checks")
    );
    assert!(
        finding_for(&report, "BYPASS_ENV_RUSTC_WORKSPACE_WRAPPER").message().contains("workspace")
    );
    assert!(
        finding_for(&report, "BYPASS_ENV_RUSTC_BOOTSTRAP").message().contains("stability gates")
    );
    Ok(())
}

#[test]
fn unset_and_empty_vars_are_clean() -> TestResult {
    assert!(scan_with(&all_unset())?.is_clean());
    assert!(scan_with(&all_set(""))?.is_clean());
    Ok(())
}

#[test]
fn rustflags_finding_names_lane_discipline_bypass() -> TestResult {
    let report = scan_with(&only("RUSTFLAGS", "-C panic=abort"))?;
    let finding = finding_for(&report, "BYPASS_ENV_RUSTFLAGS");
    assert!(finding.message().contains("RUSTFLAGS"));
    assert!(finding.message().contains("bypasses lane discipline"));
    Ok(())
}

#[test]
fn cargo_encoded_rustflags_finding_names_lane_discipline_bypass() -> TestResult {
    let report = scan_with(&only("CARGO_ENCODED_RUSTFLAGS", "-C panic=abort\0-Zlib"))?;
    let finding = finding_for(&report, "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS");
    assert!(finding.message().contains("CARGO_ENCODED_RUSTFLAGS"));
    assert!(finding.message().contains("bypasses lane discipline"));
    Ok(())
}

#[test]
fn rustc_wrapper_non_sccache_is_a_finding() -> TestResult {
    let report = scan_with(&only("RUSTC_WRAPPER", "/usr/bin/ccache"))?;
    let finding = finding_for(&report, "BYPASS_ENV_RUSTC_WRAPPER");
    assert_eq!(report.finding_count(), 1);
    assert!(finding.message().contains("bypass lane checks"));
    Ok(())
}

#[test]
fn rustc_wrapper_sccache_is_clean() -> TestResult {
    let report = scan_with(&only("RUSTC_WRAPPER", "sccache"))?;
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn rustc_wrapper_path_to_sccache_is_clean() -> TestResult {
    let report = scan_with(&only("RUSTC_WRAPPER", "/path/to/sccache"))?;
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn workspace_wrapper_and_bootstrap_have_exact_rules() -> TestResult {
    let wrapper = scan_with(&only("RUSTC_WORKSPACE_WRAPPER", "/usr/local/bin/wrapper"))?;
    assert_eq!(finding_for(&wrapper, "BYPASS_ENV_RUSTC_WORKSPACE_WRAPPER").path(), "env");

    let bootstrap = scan_with(&only("RUSTC_BOOTSTRAP", "1"))?;
    assert_eq!(finding_for(&bootstrap, "BYPASS_ENV_RUSTC_BOOTSTRAP").line(), 0);
    Ok(())
}
