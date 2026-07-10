//! Behavior tests for the Cargo.toml lint-weakening scanner.

use std::path::Path;

use tempfile::TempDir;
use titania_lanes::{Finding, LaneReport, policy_scan::cargo_lints::scan_cargo_lints_weakening};

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
fn workspace_lints_partial_table_emits_weakening_and_missing_canonical_findings() -> TestResult {
    // Strict §9.1: a partial [workspace.lints] table still enumerates
    // the missing canonical entries AND flags the inline-level weakening.
    let report = scan_manifest(
        "[workspace.lints.clippy]\nunwrap_used = { level = \"allow\", priority = -1 }\n",
    )?;
    assert!(
        report
            .findings()
            .iter()
            .any(|f| f.message().contains("workspace.lints.clippy.unwrap_used is allow")),
        "inline-level weakening finding must still fire, got: {}",
        report.render()
    );
    assert!(
        report.findings().iter().any(|f| f.message().contains("canonical lint policy missing")),
        "missing canonical entries must enumerate, got: {}",
        report.render()
    );
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

#[test]
fn root_manifest_without_workspace_table_is_clean() -> TestResult {
    // Path == Cargo.toml with no [workspace] table is a single-package crate
    // root (ManifestKind::Other), out of scope for §9.1 inheritance.
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n")?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(!found, "single-package crate root must stay clean (no applicable classification)");
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn root_manifest_with_empty_workspace_lints_table_emits_missing_canonical_finding() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", "[workspace]\n[workspace.lints]\n")?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(found, "expected empty [workspace.lints] to enumerate missing canonical entries");
    let finding = &report.findings()[0];
    assert!(finding.message().contains("canonical lint policy missing"));
    Ok(())
}

#[test]
fn member_manifest_without_lints_table_emits_missing_inheritance_finding() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "crates/foo/Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )?;
    let mut report = LaneReport::new();
    let found =
        scan_cargo_lints_weakening(dir.path(), Path::new("crates/foo/Cargo.toml"), &mut report)?;
    assert!(found, "expected member manifest without [lints] to emit a finding");
    let finding = &report.findings()[0];
    assert_eq!(finding.rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    assert_eq!(finding.path(), "crates/foo/Cargo.toml");
    assert_eq!(finding.line(), 0);
    assert!(
        finding.message().contains("workspace = true"),
        "unexpected message: {}",
        finding.message(),
    );
    Ok(())
}

#[test]
fn member_manifest_with_lints_but_no_workspace_true_emits_finding() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(&dir, "crates/foo/Cargo.toml", "[lints.clippy]\nunwrap_used = \"deny\"\n")?;
    let mut report = LaneReport::new();
    let found =
        scan_cargo_lints_weakening(dir.path(), Path::new("crates/foo/Cargo.toml"), &mut report)?;
    assert!(found, "expected member manifest missing workspace = true to emit a finding");
    let finding = &report.findings()[0];
    assert_eq!(finding.rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    assert_eq!(finding.path(), "crates/foo/Cargo.toml");
    assert_eq!(finding.line(), 0);
    assert!(finding.message().contains("workspace = true"));
    Ok(())
}

#[test]
fn member_manifest_with_workspace_true_is_clean() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "crates/foo/Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n[lints]\nworkspace = true\n",
    )?;
    let mut report = LaneReport::new();
    let found =
        scan_cargo_lints_weakening(dir.path(), Path::new("crates/foo/Cargo.toml"), &mut report)?;
    assert!(!found, "expected member with [lints] workspace = true to be clean");
    assert!(report.is_clean());
    Ok(())
}

#[test]
fn root_manifest_with_partial_workspace_lints_emits_missing_canonical_finding() -> TestResult {
    // Strict §9.1: a root with [workspace.lints] but only one entry pinned
    // is NOT clean — every EXPECTED_LEVELS entry must be pinned.
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[workspace]\n[workspace.lints.clippy]\nunwrap_used = \"deny\"\n",
    )?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(found, "partial [workspace.lints] must enumerate missing canonical entries");
    assert!(
        report
            .findings()
            .iter()
            .any(|finding| finding.message().contains("canonical lint policy missing"))
    );
    Ok(())
}

#[test]
fn policy_lint_override_weakens_required_level_and_suppresses_finding() -> TestResult {
    let dir = TempDir::new()?;
    // Manifest: clippy::needless_return = "allow" is normally a weakening
    // (default required is "deny"); without an override this must fire.
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nneedless_return = \"allow\"\nunwrap_used = \"deny\"\n",
    )?;

    // Build a PolicyProfile that downgrades needless_return to "allow".
    let profile = titania_policy::parse_profile(concat!(
        "[lints]\n",
        "\"clippy::needless_return\" = \"allow\"\n",
        "\n",
        "[architecture]\n",
        "core_dirs = [\"src/core\"]\n",
        "infra_crates = [\"tokio\"]\n",
    ))
    .expect("valid policy profile");

    let mut report = LaneReport::new();
    let _found =
        titania_lanes::policy_scan::cargo_lints::scan_cargo_lints_weakening_with_overrides(
            dir.path(),
            Path::new("Cargo.toml"),
            Some(&profile.lints),
            &mut report,
        )?;

    assert!(
        report.is_clean(),
        "expected the override to suppress the needless_return weakening, got: {}",
        report.render(),
    );
    Ok(())
}

#[test]
fn policy_lint_override_only_weakens_never_strengthens() -> TestResult {
    let dir = TempDir::new()?;
    // Manifest: clippy::unwrap_used = "deny" satisfies the default.
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"deny\"\nneedless_return = \"deny\"\n",
    )?;

    // A profile that *strengthens* needless_return from "deny" to "forbid"
    // must NOT be honored — overrides only weaken the required level.
    let profile = titania_policy::parse_profile(concat!(
        "[lints]\n",
        "\"clippy::needless_return\" = \"forbid\"\n",
        "\n",
        "[architecture]\n",
        "core_dirs = [\"src/core\"]\n",
        "infra_crates = [\"tokio\"]\n",
    ))
    .expect("valid policy profile");

    let mut report = LaneReport::new();
    let _found =
        titania_lanes::policy_scan::cargo_lints::scan_cargo_lints_weakening_with_overrides(
            dir.path(),
            Path::new("Cargo.toml"),
            Some(&profile.lints),
            &mut report,
        )?;

    // The default "deny" requirement still holds; a manifest with "deny"
    // remains clean. The strengthening override is silently ignored.
    assert!(report.is_clean(), "expected clean report, got: {}", report.render());
    Ok(())
}

/// Number of canonical entries in `EXPECTED_LEVELS`; exact-count tests
/// below assert that every absent canonical emits its own typed finding.
const CANONICAL_LINT_COUNT: usize = 16;

#[test]
fn empty_workspace_lints_table_enumerates_every_canonical_omission() -> TestResult {
    // Strict §9.1: a root with `[workspace]` but an empty
    // `[workspace.lints]` table must surface one typed
    // `BYPASS_CARGO_LINTS_WEAKENING` finding per absent canonical
    // entry, not just one sentinel or one finding.
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", "[workspace]\n[workspace.lints]\n")?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(found, "expected empty [workspace.lints] to enumerate missing canonical entries");
    let canonical_findings: Vec<&Finding> = report
        .findings()
        .iter()
        .filter(|finding| finding.message().contains("canonical lint policy missing"))
        .collect();
    assert_eq!(
        canonical_findings.len(),
        CANONICAL_LINT_COUNT,
        "expected one canonical finding per EXPECTED_LEVELS entry, got {} (full report: {})",
        canonical_findings.len(),
        report.render(),
    );
    for finding in &canonical_findings {
        assert_eq!(finding.rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
        assert_eq!(finding.path(), "Cargo.toml");
        assert!(finding.message().contains("does not pin"));
        assert!(finding.message().contains("required deny"));
    }
    Ok(())
}

#[test]
fn partial_workspace_lints_table_enumerates_every_remaining_canonical_omission() -> TestResult {
    // Strict §9.1: a root with `[workspace]` and a partially pinned
    // `[workspace.lints]` table must surface one typed
    // `BYPASS_CARGO_LINTS_WEAKENING` finding per still-absent canonical
    // entry — pinning one entry does not excuse the others.
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[workspace]\n[workspace.lints.clippy]\nunwrap_used = \"deny\"\n",
    )?;
    let mut report = LaneReport::new();
    let found = scan_cargo_lints_weakening(dir.path(), Path::new("Cargo.toml"), &mut report)?;
    assert!(found, "partial [workspace.lints] must enumerate missing canonical entries");
    let canonical_findings: Vec<&Finding> = report
        .findings()
        .iter()
        .filter(|finding| finding.message().contains("canonical lint policy missing"))
        .collect();
    assert_eq!(
        canonical_findings.len(),
        CANONICAL_LINT_COUNT - 1,
        "pinning clippy::unwrap_used must still leave {} canonical omissions, got {} (full report: {})",
        CANONICAL_LINT_COUNT - 1,
        canonical_findings.len(),
        report.render(),
    );
    // No canonical finding should target the pinned entry.
    for finding in &canonical_findings {
        assert!(
            !finding.message().contains("clippy.unwrap_used"),
            "pinned entry must not be reported as missing: {}",
            finding.message(),
        );
    }
    Ok(())
}
