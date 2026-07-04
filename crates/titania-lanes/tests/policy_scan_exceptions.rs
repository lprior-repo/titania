//! Behavior tests for policy-scan exception filtering and exception parsing.
//!
//! Proves:
//! (a) A valid exception suppresses a matching Cargo.toml lint-weakening finding.
//! (b) A non-matching path or rule does not suppress.
//! (c) `parse_exceptions` with an expired entry yields `ExceptionError::code() == "POLICY_EXCEPTION_EXPIRED"`.

use tempfile::TempDir;
use titania_core::{RuleId, WorkspacePath};
use titania_lanes::{LaneReport, policy_scan::scan_policy_inputs_with_exceptions};
use titania_policy::{Exception, ExceptionError, parse_exceptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_manifest(dir: &TempDir, name: &str, content: &str) -> std::io::Result<()> {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

/// Build a minimal valid [`Exception`] for testing suppression.
fn make_exception(rule: &str, path: &str) -> Exception {
    Exception {
        rule_id: RuleId::new(rule).expect("valid rule id"),
        path: WorkspacePath::new(path).expect("valid workspace path"),
        owner: "test-team".into(),
        reason: "test justification".into(),
        expires_on: "2099-12-31".into(),
        review: "TICKET-1".into(),
    }
}

/// Run policy scan with exceptions and return findings that remain after suppression.
fn scan_with_exceptions(
    dir: &TempDir,
    manifest_name: &str,
    exceptions: &[Exception],
) -> Result<LaneReport, Box<dyn std::error::Error>> {
    let manifest_path = dir.path().join(manifest_name);
    let mut report = LaneReport::new();
    scan_policy_inputs_with_exceptions(
        dir.path(),
        std::iter::once(manifest_path.as_path()),
        exceptions,
        &mut report,
    )?;
    Ok(report)
}

// ─── (a) Valid exception suppresses a matching finding ───────────────────────

#[test]
fn matching_exception_suppresses_lint_weakening() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n",
    )?;
    let exception = make_exception("BYPASS_CARGO_LINTS_WEAKENING", "Cargo.toml");
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[exception])?;
    assert!(report.is_clean(), "expected suppression but found: {}", report.render());
    Ok(())
}

#[test]
fn matching_exception_suppresses_config_flags_finding() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(&dir, "Cargo.toml", "[workspace]\nmembers = [\"crates/example\"]\n")?;
    std::fs::create_dir_all(dir.path().join(".cargo"))?;
    write_manifest(&dir, ".cargo/config.toml", "[build]\nrustflags = [\"-Z\", \"some-flag\"]\n")?;
    let exception = make_exception("BYPASS_CARGO_CONFIG_FLAGS", ".cargo/config.toml");
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[exception])?;
    assert!(report.is_clean(), "expected suppression but found: {}", report.render());
    Ok(())
}

// ─── (b) Non-matching path or rule does NOT suppress ─────────────────────────

#[test]
fn non_matching_rule_does_not_suppress() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n",
    )?;
    // Exception for a different rule — should NOT suppress.
    let exception = make_exception("BYPASS_CARGO_CONFIG_FLAGS", "Cargo.toml");
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[exception])?;
    assert_eq!(report.finding_count(), 1, "expected one finding, got {}", report.render());
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    assert!(report.findings()[0].message().contains("unwrap_used"));
    Ok(())
}

#[test]
fn non_matching_path_does_not_suppress() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n",
    )?;
    // Exception for the right rule but wrong path — should NOT suppress.
    let exception = make_exception("BYPASS_CARGO_LINTS_WEAKENING", "other/Cargo.toml");
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[exception])?;
    assert_eq!(report.finding_count(), 1, "expected one finding, got {}", report.render());
    assert_eq!(report.findings()[0].path(), "Cargo.toml");
    Ok(())
}

#[test]
fn exception_matches_nothing_when_both_rule_and_path_wrong() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n",
    )?;
    let exception = make_exception("BYPASS_CARGO_CONFIG_WRAPPER", "other/file.toml");
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[exception])?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    Ok(())
}

#[test]
fn empty_exceptions_suppress_nothing() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n",
    )?;
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[])?;
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    Ok(())
}

// ─── (c) Expired exception parsing yields POLICY_EXCEPTION_EXPIRED ───────────

#[test]
fn parse_expired_exception_returns_expired_error() -> TestResult {
    // Today is 2026-07-04. An exception that expired on 2025-01-01 is stale.
    let today = "2026-07-04";
    let expired_content = r#"
[[exceptions]]
rule_id = "BYPASS_CARGO_LINTS_WEAKENING"
path = "Cargo.toml"
owner = "test-team"
reason = "temporary workaround"
expires_on = "2025-01-01"
review = "TICKET-42"
"#;
    match parse_exceptions(expired_content, today) {
        Err(ExceptionError::ExceptionExpired { rule_id, expires_on, today: today_val }) => {
            assert_eq!(
                ExceptionError::ExceptionExpired { rule_id, expires_on, today: today_val }.code(),
                "POLICY_EXCEPTION_EXPIRED"
            );
        }
        other => panic!("expected ExceptionExpired error, got: {:?}", other),
    }
    Ok(())
}

#[test]
fn valid_exception_does_not_expire_before_today() -> TestResult {
    // Exception that expires tomorrow — should NOT be expired.
    let today = "2026-07-04";
    let content = r#"
[[exceptions]]
rule_id = "BYPASS_CARGO_LINTS_WEAKENING"
path = "Cargo.toml"
owner = "team"
reason = "planned"
expires_on = "2026-07-05"
review = "TICKET-1"
"#;
    let exceptions = parse_exceptions(content, today).expect("should not be expired");
    assert_eq!(exceptions.len(), 1);
    assert_eq!(exceptions[0].rule_id.as_str(), "BYPASS_CARGO_LINTS_WEAKENING");
    assert_eq!(exceptions[0].expires_on.as_ref(), "2026-07-05");
    Ok(())
}

#[test]
fn same_day_expires_on_is_valid_not_expired() -> TestResult {
    // v1: expired only when expires_on < today; same-day remains valid.
    let today = "2026-07-04";
    let content = r#"
[[exceptions]]
rule_id = "BYPASS_CARGO_LINTS_WEAKENING"
path = "Cargo.toml"
owner = "team"
reason = "expiring today"
expires_on = "2026-07-04"
review = "TICKET-1"
"#;
    let exceptions = parse_exceptions(content, today).expect("same-day expiry is valid");
    assert_eq!(exceptions.len(), 1);
    assert_eq!(exceptions[0].expires_on.as_ref(), "2026-07-04");
    Ok(())
}
