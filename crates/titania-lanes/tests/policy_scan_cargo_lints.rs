//! Behavior tests for the Cargo.toml lint-weakening scanner.

use std::path::Path;

use tempfile::TempDir;
use titania_lanes::{LaneReport, policy_scan::cargo_lints::scan_cargo_lints_weakening};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_manifest(dir: &TempDir, name: &str, content: &str) -> std::io::Result<()> {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

fn scan_manifest(content: &str) -> Result<LaneReport, Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", content)?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(found, "expected at least one lint weakening finding");
    Ok(report)
}

fn assert_one_finding(report: &LaneReport, line: u32, prefix: &str) {
    assert_eq!(report.finding_count(), 1, "expected one finding, got {}", report.render());
    let finding = &report.findings()[0];
    assert_eq!(finding.rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    assert_eq!(finding.path(), "Cargo.toml");
    assert_eq!(finding.line(), line);
    assert!(finding.message().starts_with(prefix), "unexpected message: {}", finding.message());
}

#[test]
fn clippy_allow_weakenings_emit_typed_findings() -> TestResult {
    let report = scan_manifest("[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n")?;
    assert_one_finding(&report, 2, "clippy.unwrap_used is allow");
    Ok(())
}

#[test]
fn clippy_warn_weakenings_emit_typed_findings() -> TestResult {
    let report = scan_manifest("[lints.clippy]\npedantic = \"warn\"\nunwrap_used = \"deny\"\n")?;
    assert_one_finding(&report, 2, "clippy.pedantic is warn");
    Ok(())
}

#[test]
fn rust_allow_weakenings_emit_typed_findings() -> TestResult {
    let report =
        scan_manifest("[lints.rust]\nunsafe_code = \"allow\"\nunreachable_pub = \"deny\"\n")?;
    assert_one_finding(&report, 2, "rust.unsafe_code is allow");
    Ok(())
}

#[test]
fn multiple_weakened_lints_produce_one_finding_each() -> TestResult {
    let report = scan_manifest(
        "[lints.clippy]\nunwrap_used = \"allow\"\nexpect_used = \"warn\"\npanic = \"deny\"\n",
    )?;
    assert_eq!(report.finding_count(), 2);
    assert!(report.findings()[0].message().starts_with("clippy.unwrap_used is allow"));
    assert_eq!(report.findings()[0].line(), 2);
    assert!(report.findings()[1].message().starts_with("clippy.expect_used is warn"));
    assert_eq!(report.findings()[1].line(), 3);
    Ok(())
}

#[test]
fn required_deny_and_warn_levels_are_clean() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"deny\"\npanic = \"deny\"\n\n[lints.rust]\ndeprecated = \"warn\"\nunsafe_code = \"deny\"\n",
    )?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(!found);
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn manifests_without_lints_are_not_applicable() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n")?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(!found);
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn subdirectory_manifest_uses_manifest_path_in_finding() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(&dir, "crates/foo/Cargo.toml", "[lints.clippy]\nunwrap_used = \"allow\"\n")?;
    let mut report = LaneReport::new();
    let found =
        scan_cargo_lints_weakening(dir.path(), Path::new("crates/foo/Cargo.toml"), &mut report)?;
    assert!(found);
    assert_eq!(report.findings()[0].path(), "crates/foo/Cargo.toml");
    Ok(())
}

#[test]
fn workspace_lints_and_inline_level_tables_are_scanned() -> TestResult {
    let report = scan_manifest(
        "[workspace.lints.clippy]\nunwrap_used = { level = \"allow\", priority = -1 }\n",
    )?;
    assert_one_finding(&report, 2, "workspace.lints.clippy.unwrap_used is allow");
    Ok(())
}

#[test]
fn malformed_toml_is_not_a_weakening_finding() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", "[lints.clippy]\nunwrap_used\npanic = \"deny\"\n")?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(!found);
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn message_contains_actual_required_and_weakened_terms() -> TestResult {
    let report = scan_manifest("[lints.clippy]\nunwrap_used = \"allow\"\n")?;
    let message = report.findings()[0].message();
    assert!(message.contains("clippy.unwrap_used"));
    assert!(message.contains("allow"));
    assert!(message.contains("deny"));
    assert!(message.contains("weakened"));
    Ok(())
}
