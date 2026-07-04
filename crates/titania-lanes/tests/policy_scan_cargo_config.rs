//! Behavior tests for the `.cargo/config` policy scanner.

use std::path::Path;

use tempfile::TempDir;
use titania_lanes::{LaneReport, policy_scan::cargo_config::scan_cargo_config_from};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_file(dir: &Path, rel: &str, content: &str) -> std::io::Result<()> {
    let full = dir.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full, content)
}

fn scan_from(path: &Path) -> Result<LaneReport, Box<dyn std::error::Error>> {
    let mut report = LaneReport::new();
    scan_cargo_config_from(path, &mut report)?;
    Ok(report)
}

fn fixture() -> std::io::Result<TempDir> {
    let tmp = tempfile::tempdir()?;
    write_file(tmp.path(), "Cargo.toml", "[workspace]\nmembers = [\"crates/example\"]\n")?;
    write_file(
        tmp.path(),
        "crates/example/Cargo.toml",
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\n",
    )?;
    Ok(tmp)
}

#[test]
fn config_toml_rustflags_emit_one_finding_per_flag() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config.toml", "[build]\nrustflags = [\"-Z\", \"some-flag\"]\n")?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 2);
    assert!(report.findings().iter().all(|f| f.rule().as_str() == "BYPASS_CARGO_CONFIG_FLAGS"));
    assert!(report.findings()[0].message().contains("-Z"));
    assert!(report.findings()[1].message().contains("some-flag"));
    Ok(())
}

#[test]
fn extensionless_config_rustflags_are_scanned() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config", "[build]\nrustflags = \"-D warnings\"\n")?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].path(), ".cargo/config");
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_CONFIG_FLAGS");
    Ok(())
}

#[test]
fn empty_rustflags_and_non_build_rustflags_are_clean() -> TestResult {
    let empty = fixture()?;
    write_file(empty.path(), ".cargo/config.toml", "[build]\nrustflags = []\n")?;
    assert!(scan_from(empty.path())?.is_clean());

    let target = fixture()?;
    write_file(
        target.path(),
        ".cargo/config.toml",
        "[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-C\", \"linker=lld\"]\n",
    )?;
    assert!(scan_from(target.path())?.is_clean());
    Ok(())
}

#[test]
fn sccache_wrappers_are_clean() -> TestResult {
    let tmp = fixture()?;
    write_file(
        tmp.path(),
        ".cargo/config.toml",
        "[build]\nrustc-wrapper = \"sccache\"\nworkspace-wrapper = \"sccache\"\n",
    )?;
    assert!(scan_from(tmp.path())?.is_clean());
    Ok(())
}

#[test]
fn non_sccache_wrappers_emit_wrapper_findings() -> TestResult {
    let tmp = fixture()?;
    write_file(
        tmp.path(),
        ".cargo/config.toml",
        "[build]\nrustc-wrapper = \"ccache\"\nworkspace-wrapper = \"distccd\"\n",
    )?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 2);
    assert!(report.findings().iter().all(|f| f.rule().as_str() == "BYPASS_CARGO_CONFIG_WRAPPER"));
    assert!(report.findings()[0].message().contains("ccache"));
    assert!(report.findings()[1].message().contains("distccd"));
    Ok(())
}

#[test]
fn absent_cargo_config_files_are_clean() -> TestResult {
    let no_cargo_dir = fixture()?;
    assert!(scan_from(no_cargo_dir.path())?.is_clean());

    let empty_cargo_dir = fixture()?;
    std::fs::create_dir_all(empty_cargo_dir.path().join(".cargo"))?;
    assert!(scan_from(empty_cargo_dir.path())?.is_clean());
    Ok(())
}

#[test]
fn both_config_files_are_scanned_in_stable_order() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config", "[build]\nrustc-wrapper = \"ccache\"\n")?;
    write_file(tmp.path(), ".cargo/config.toml", "[build]\nrustc-wrapper = \"distccd\"\n")?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 2);
    assert_eq!(report.findings()[0].path(), ".cargo/config");
    assert_eq!(report.findings()[1].path(), ".cargo/config.toml");
    Ok(())
}

#[test]
fn mixed_rustflags_and_wrapper_findings_share_file_path() -> TestResult {
    let tmp = fixture()?;
    write_file(
        tmp.path(),
        ".cargo/config.toml",
        "[build]\nrustc-wrapper = \"ccache\"\nrustflags = [\"-Z\", \"unstable\"]\n",
    )?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 3);
    assert!(report.findings().iter().all(|f| f.path() == ".cargo/config.toml"));
    assert_eq!(
        report
            .findings()
            .iter()
            .filter(|finding| finding.rule().as_str() == "BYPASS_CARGO_CONFIG_FLAGS")
            .count(),
        2
    );
    Ok(())
}

#[test]
fn parent_cargo_config_is_scanned_from_child_crate() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config.toml", "[build]\nrustc-wrapper = \"sccache\"\n")?;
    let child = tmp.path().join("crates/example");
    let report = scan_from(&child)?;
    assert_eq!(report.finding_count(), 1);
    assert!(report.findings()[0].path().ends_with(".cargo/config.toml"));
    Ok(())
}

#[test]
fn local_sccache_wrapper_in_child_is_clean() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config.toml", "[build]\nrustc-wrapper = \"ccache\"\n")?;
    write_file(
        tmp.path(),
        "crates/example/.cargo/config.toml",
        "[build]\nrustc-wrapper = \"sccache\"\n",
    )?;
    let child = tmp.path().join("crates/example");
    let report = scan_from(&child)?;
    assert_eq!(report.finding_count(), 1);
    assert!(report.findings()[0].path().ends_with(".cargo/config.toml"));
    assert!(report.findings()[0].message().contains("parent-directory Cargo config is rejected"));
    Ok(())
}
