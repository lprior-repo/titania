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
fn empty_target_tables_are_clean_but_target_rustflags_are_findings() -> TestResult {
    let empty = fixture()?;
    write_file(empty.path(), ".cargo/config.toml", "[target.'cfg(unix)']\n")?;
    assert!(scan_from(empty.path())?.is_clean());

    let target_string = fixture()?;
    write_file(
        target_string.path(),
        ".cargo/config.toml",
        "[target.'cfg(unix)']\nrustflags = \"-C debuginfo=0\"\n",
    )?;
    let string_report = scan_from(target_string.path())?;
    assert_eq!(string_report.finding_count(), 1);
    assert!(string_report.findings()[0].message().contains("debuginfo"));

    let target_array = fixture()?;
    write_file(
        target_array.path(),
        ".cargo/config.toml",
        "[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-Z\", \"unstable\"]\n",
    )?;
    let array_report = scan_from(target_array.path())?;
    assert_eq!(array_report.finding_count(), 2);
    assert!(
        array_report.findings().iter().all(|f| f.rule().as_str() == "BYPASS_CARGO_CONFIG_FLAGS")
    );
    Ok(())
}

#[test]
fn inline_build_table_rustflags_emit_flag_findings() -> TestResult {
    let tmp = fixture()?;
    write_file(
        tmp.path(),
        ".cargo/config.toml",
        "build = { rustflags = [\"-Z\", \"unstable\"] }\n",
    )?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 2);
    assert!(report.findings().iter().all(|f| f.rule().as_str() == "BYPASS_CARGO_CONFIG_FLAGS"));
    assert!(report.findings()[0].message().contains("-Z"));
    assert!(report.findings()[1].message().contains("unstable"));
    Ok(())
}

#[test]
fn inline_target_table_rustflags_emit_flag_findings() -> TestResult {
    let tmp = fixture()?;
    write_file(
        tmp.path(),
        ".cargo/config.toml",
        "target = { \"x86_64-unknown-linux-gnu\" = { rustflags = [\"-Z\"] } }\n",
    )?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_CONFIG_FLAGS");
    assert!(report.findings()[0].message().contains("-Z"));
    Ok(())
}

#[test]
fn inline_target_table_with_cfg_key_is_clean() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config.toml", "target = { \"cfg(unix)\" = {} }\n")?;
    assert!(scan_from(tmp.path())?.is_clean());
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

#[test]
fn malformed_cargo_config_emits_parse_error_finding() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config.toml", "this is = not valid toml ]\n")?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_CONFIG_PARSE_ERROR");
    assert_eq!(report.findings()[0].path(), ".cargo/config.toml");
    assert!(report.findings()[0].message().contains("malformed Cargo config"));
    Ok(())
}

#[test]
fn extensionless_malformed_cargo_config_still_parses_to_parse_error() -> TestResult {
    let tmp = fixture()?;
    write_file(tmp.path(), ".cargo/config", "[unterminated section\n")?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_CONFIG_PARSE_ERROR");
    assert_eq!(report.findings()[0].path(), ".cargo/config");
    Ok(())
}

#[test]
fn unreadable_cargo_config_emits_read_error_finding() -> TestResult {
    let tmp = fixture()?;
    let config_path = tmp.path().join(".cargo/config.toml");
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Invalid UTF-8 bytes force `read_to_string` to fail with `InvalidData`,
    // which is a deterministic, cross-platform read error.
    std::fs::write(&config_path, [0xFF_u8, 0xFE, 0xFD])?;
    let report = scan_from(tmp.path())?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_CONFIG_READ_ERROR");
    assert_eq!(report.findings()[0].path(), ".cargo/config.toml");
    assert!(report.findings()[0].message().contains("cannot read Cargo config file"));
    Ok(())
}

#[test]
fn absent_cargo_config_stays_clean_when_siblings_are_present() -> TestResult {
    let tmp = fixture()?;
    // Empty `.cargo/` directory: no `config` or `config.toml` present.
    std::fs::create_dir_all(tmp.path().join(".cargo"))?;
    let report = scan_from(tmp.path())?;
    assert!(report.is_clean(), "expected clean report, got: {}", report.render());
    Ok(())
}
