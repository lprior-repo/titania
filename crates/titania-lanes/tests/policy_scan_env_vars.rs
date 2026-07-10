//! Behavior tests for the environment-variable policy scanner.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use tempfile::tempdir;
use titania_lanes::{
    LaneReport, RuleIdError,
    policy_scan::env_vars::{
        CONTROLLED_CARGO_HOME_SUFFIX, CONTROLLED_RUSTUP_HOME_SUFFIX, controlled_home_path,
        scan_env_vars_with_target,
    },
};

use titania_lanes::policy_scan::env_vars::scan_env_vars_with;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Build a `PathBuf` from a controlled-home suffix without panicking.
fn expected_home(root: &Path, suffix: &str) -> PathBuf {
    controlled_home_path(root, suffix)
}

fn scan_target_with(
    map: &BTreeMap<&str, Option<&str>>,
    root: &Path,
) -> Result<LaneReport, RuleIdError> {
    let owned = map
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.map(ToOwned::to_owned)))
        .collect::<BTreeMap<String, Option<String>>>();
    let env = move |name: &str| owned.get(name).cloned().flatten();
    let mut report = LaneReport::new();
    scan_env_vars_with_target(&mut report, &env, root)?;
    Ok(report)
}

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
fn unset_vars_are_clean_but_empty_bootstrap_is_forbidden() -> TestResult {
    assert!(scan_with(&all_unset())?.is_clean());
    let report = scan_with(&all_set(""))?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(finding_for(&report, "BYPASS_ENV_RUSTC_BOOTSTRAP").line(), 0);
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

/// Strict controlled-home contract: an absent `CARGO_HOME` / `RUSTUP_HOME`
/// must emit one finding per variable, tagged with the v1-spec §8 rule ids
/// `BYPASS_ENV_CARGO_HOME` and `BYPASS_ENV_RUSTUP_HOME`, and must name the
/// controlled path so callers can fix the environment.
#[test]
fn absent_cargo_home_and_rustup_home_emit_controlled_findings() -> TestResult {
    let tmp = tempdir()?;
    let root = tmp.path();
    let controlled_cargo = expected_home(root, CONTROLLED_CARGO_HOME_SUFFIX);
    let controlled_rustup = expected_home(root, CONTROLLED_RUSTUP_HOME_SUFFIX);

    let report = scan_target_with(&BTreeMap::new(), root)?;

    assert_eq!(report.finding_count(), 2);
    let cargo_finding = finding_for(&report, "BYPASS_ENV_CARGO_HOME");
    assert_eq!(cargo_finding.path(), "env");
    assert_eq!(cargo_finding.line(), 0);
    assert!(
        cargo_finding.message().contains("CARGO_HOME"),
        "cargo finding must name the variable: {}",
        cargo_finding.message(),
    );
    assert!(
        cargo_finding.message().contains(controlled_cargo.to_str().expect("utf-8 path")),
        "cargo finding must name the controlled path: {}",
        cargo_finding.message(),
    );
    assert!(
        cargo_finding.message().contains("must be set"),
        "absent cargo-home finding must mention 'must be set': {}",
        cargo_finding.message(),
    );

    let rustup_finding = finding_for(&report, "BYPASS_ENV_RUSTUP_HOME");
    assert_eq!(rustup_finding.path(), "env");
    assert_eq!(rustup_finding.line(), 0);
    assert!(
        rustup_finding.message().contains("RUSTUP_HOME"),
        "rustup finding must name the variable: {}",
        rustup_finding.message(),
    );
    assert!(
        rustup_finding.message().contains(controlled_rustup.to_str().expect("utf-8 path")),
        "rustup finding must name the controlled path: {}",
        rustup_finding.message(),
    );
    assert!(
        rustup_finding.message().contains("must be set"),
        "absent rustup-home finding must mention 'must be set': {}",
        rustup_finding.message(),
    );
    Ok(())
}

/// Empty string values are still violations — they pass `AnyNonEmptyForbidden`
/// (set) but fail the strict controlled-home equality check.
#[test]
fn empty_cargo_home_and_rustup_home_emit_controlled_findings() -> TestResult {
    let tmp = tempdir()?;
    let root = tmp.path();
    let controlled_cargo = expected_home(root, CONTROLLED_CARGO_HOME_SUFFIX);

    let env_map = BTreeMap::from([("CARGO_HOME", Some("")), ("RUSTUP_HOME", Some(""))]);
    let report = scan_target_with(&env_map, root)?;

    assert_eq!(report.finding_count(), 2);
    let cargo_finding = finding_for(&report, "BYPASS_ENV_CARGO_HOME");
    assert_eq!(cargo_finding.line(), 0);
    assert!(
        cargo_finding.message().contains("non-empty"),
        "empty cargo-home finding must call out the empty value: {}",
        cargo_finding.message(),
    );
    assert!(
        cargo_finding.message().contains(controlled_cargo.to_str().expect("utf-8 path")),
        "empty cargo-home finding must name the controlled path: {}",
        cargo_finding.message(),
    );

    let rustup_finding = finding_for(&report, "BYPASS_ENV_RUSTUP_HOME");
    assert_eq!(rustup_finding.line(), 0);
    assert!(
        rustup_finding.message().contains("RUSTUP_HOME"),
        "empty rustup-home finding must name the variable: {}",
        rustup_finding.message(),
    );
    Ok(())
}

/// A value that points anywhere other than `<root>/.titania/hermetic/<suffix>`
/// is a violation; the finding must surface both the expected controlled path
/// and the offending value so the operator can correct it.
#[test]
fn wrong_rooted_cargo_home_and_rustup_home_emit_controlled_findings() -> TestResult {
    let tmp = tempdir()?;
    let root = tmp.path();
    let controlled_cargo = expected_home(root, CONTROLLED_CARGO_HOME_SUFFIX);
    let controlled_rustup = expected_home(root, CONTROLLED_RUSTUP_HOME_SUFFIX);
    let wrong_cargo = "/var/tmp/untrusted-cargo-home";
    let wrong_rustup = "/var/tmp/untrusted-rustup-home";

    let env_map =
        BTreeMap::from([("CARGO_HOME", Some(wrong_cargo)), ("RUSTUP_HOME", Some(wrong_rustup))]);
    let report = scan_target_with(&env_map, root)?;

    assert_eq!(report.finding_count(), 2);
    let cargo_finding = finding_for(&report, "BYPASS_ENV_CARGO_HOME");
    assert!(
        cargo_finding.message().contains(wrong_cargo),
        "wrong-root cargo finding must echo the offending value: {}",
        cargo_finding.message(),
    );
    assert!(
        cargo_finding.message().contains(controlled_cargo.to_str().expect("utf-8 path")),
        "wrong-root cargo finding must name the controlled path: {}",
        cargo_finding.message(),
    );

    let rustup_finding = finding_for(&report, "BYPASS_ENV_RUSTUP_HOME");
    assert!(
        rustup_finding.message().contains(wrong_rustup),
        "wrong-root rustup finding must echo the offending value: {}",
        rustup_finding.message(),
    );
    assert!(
        rustup_finding.message().contains(controlled_rustup.to_str().expect("utf-8 path")),
        "wrong-root rustup finding must name the controlled path: {}",
        rustup_finding.message(),
    );
    Ok(())
}

/// Setting both `CARGO_HOME` and `RUSTUP_HOME` to the strict target-root
/// controlled paths — and leaving every other lane-bypass variable unset —
/// must produce a clean report.
#[test]
fn exact_controlled_paths_pass_strict_check() -> TestResult {
    let tmp = tempdir()?;
    let root = tmp.path();
    let controlled_cargo = expected_home(root, CONTROLLED_CARGO_HOME_SUFFIX);
    let controlled_rustup = expected_home(root, CONTROLLED_RUSTUP_HOME_SUFFIX);

    let env_map = BTreeMap::from([
        ("CARGO_HOME", Some(controlled_cargo.to_str().expect("utf-8 cargo path"))),
        ("RUSTUP_HOME", Some(controlled_rustup.to_str().expect("utf-8 rustup path"))),
    ]);
    let report = scan_target_with(&env_map, root)?;

    assert!(
        report.is_clean(),
        "controlled homes must be clean; got findings: {:?}",
        report.findings().iter().map(|f| (f.rule().as_str(), f.message())).collect::<Vec<_>>(),
    );
    assert_eq!(report.finding_count(), 0);
    Ok(())
}

/// Controlled-home correctness must not mask the standalone
/// `RUSTC_BOOTSTRAP` ban. Even when the homes are exactly right, an empty
/// `RUSTC_BOOTSTRAP` is still a finding.
#[test]
fn empty_bootstrap_remains_a_finding_under_strict_homes() -> TestResult {
    let tmp = tempdir()?;
    let root = tmp.path();
    let controlled_cargo = expected_home(root, CONTROLLED_CARGO_HOME_SUFFIX);
    let controlled_rustup = expected_home(root, CONTROLLED_RUSTUP_HOME_SUFFIX);

    let env_map = BTreeMap::from([
        ("CARGO_HOME", Some(controlled_cargo.to_str().expect("utf-8 cargo path"))),
        ("RUSTUP_HOME", Some(controlled_rustup.to_str().expect("utf-8 rustup path"))),
        ("RUSTC_BOOTSTRAP", Some("")),
    ]);
    let report = scan_target_with(&env_map, root)?;

    assert_eq!(report.finding_count(), 1);
    let bootstrap_finding = finding_for(&report, "BYPASS_ENV_RUSTC_BOOTSTRAP");
    assert_eq!(bootstrap_finding.path(), "env");
    assert_eq!(bootstrap_finding.line(), 0);
    assert!(
        bootstrap_finding.message().contains("stability gates"),
        "bootstrap finding must cite stability gates: {}",
        bootstrap_finding.message(),
    );
    Ok(())
}

/// Each controlled-home check is independent: a wrong `CARGO_HOME` alone
/// must produce exactly one finding, leaving the rustup-side clean. This
/// guards against the two checks accidentally sharing state.
#[test]
fn wrong_cargo_home_with_correct_rustup_home_is_singular() -> TestResult {
    let tmp = tempdir()?;
    let root = tmp.path();
    let controlled_rustup = expected_home(root, CONTROLLED_RUSTUP_HOME_SUFFIX);

    let env_map = BTreeMap::from([
        ("CARGO_HOME", Some("/var/tmp/untrusted-cargo-home")),
        ("RUSTUP_HOME", Some(controlled_rustup.to_str().expect("utf-8 rustup path"))),
    ]);
    let report = scan_target_with(&env_map, root)?;

    assert_eq!(report.finding_count(), 1);
    assert!(
        finding_for(&report, "BYPASS_ENV_CARGO_HOME").rule().as_str() == "BYPASS_ENV_CARGO_HOME"
    );
    assert!(
        report.findings().iter().all(|f| f.rule().as_str() != "BYPASS_ENV_RUSTUP_HOME"),
        "rustup-home must not be flagged when its value matches the controlled path",
    );
    Ok(())
}
