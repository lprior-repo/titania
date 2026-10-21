use tempfile::TempDir;
use titania_core::{RuleId, WorkspacePath};
use titania_lanes::{
    LaneReport,
    policy_scan::{
        env_vars::{
            CONTROLLED_CARGO_HOME_SUFFIX, CONTROLLED_RUSTUP_HOME_SUFFIX, controlled_home_path,
        },
        scan_policy_inputs_with_exceptions,
    },
};
use titania_policy::{Exception, ExceptionError, parse_exceptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_manifest(dir: &TempDir, name: &str, content: &str) -> std::io::Result<()> {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

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

fn controlled_env(dir: &TempDir) -> std::sync::Arc<Box<dyn Fn(&str) -> Option<String>>> {
    let cargo = controlled_home_path(dir.path(), CONTROLLED_CARGO_HOME_SUFFIX)
        .into_os_string()
        .into_string()
        .expect("controlled cargo-home path is UTF-8");
    let rustup = controlled_home_path(dir.path(), CONTROLLED_RUSTUP_HOME_SUFFIX)
        .into_os_string()
        .into_string()
        .expect("controlled rustup-home path is UTF-8");
    let map: std::collections::BTreeMap<String, String> =
        [("CARGO_HOME".to_owned(), cargo), ("RUSTUP_HOME".to_owned(), rustup)]
            .into_iter()
            .collect();
    let env: Box<dyn Fn(&str) -> Option<String>> =
        Box::new(move |name: &str| map.get(name).cloned());
    std::sync::Arc::new(env)
}

fn scan_with_exceptions(
    dir: &TempDir,
    manifest_name: &str,
    exceptions: &[Exception],
) -> Result<LaneReport, Box<dyn std::error::Error>> {
    let manifest_path = dir.path().join(manifest_name);
    let mut report = LaneReport::new();
    let env = controlled_env(dir);
    scan_policy_inputs_with_exceptions(
        dir.path(),
        std::iter::once(manifest_path.as_path()),
        exceptions,
        env.as_ref(),
        &mut report,
    )?;
    Ok(report)
}

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
    write_manifest(
        &dir,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/example\"]\n[workspace.lints.rust]\nwarnings = \"deny\"\nfuture_incompatible = \"deny\"\nrust_2018_idioms = \"deny\"\nunsafe_code = \"forbid\"\nunsafe_op_in_unsafe_fn = \"deny\"\nunused_must_use = \"deny\"\nunused_results = \"deny\"\nlet_underscore_drop = \"deny\"\nelided_lifetimes_in_paths = \"deny\"\nexplicit_outlives_requirements = \"deny\"\nmissing_debug_implementations = \"deny\"\nunreachable_pub = \"deny\"\ntrivial_casts = \"deny\"\ntrivial_numeric_casts = \"deny\"\nvariant_size_differences = \"deny\"\nunused_extern_crates = \"deny\"\nunused_import_braces = \"deny\"\nkeyword_idents_2024 = \"deny\"\n[workspace.lints.rustdoc]\nbare_urls = \"deny\"\nbroken_intra_doc_links = \"deny\"\nprivate_intra_doc_links = \"deny\"\n[workspace.lints.clippy]\nall = \"deny\"\ncargo = \"deny\"\npedantic = \"deny\"\nnursery = \"deny\"\nallow_attributes = \"deny\"\nallow_attributes_without_reason = \"deny\"\nunwrap_used = \"deny\"\nexpect_used = \"deny\"\nunwrap_in_result = \"deny\"\npanic = \"deny\"\npanic_in_result_fn = \"deny\"\ntodo = \"deny\"\nunimplemented = \"deny\"\nunreachable = \"deny\"\ndbg_macro = \"deny\"\nprint_stdout = \"deny\"\nprint_stderr = \"deny\"\nindexing_slicing = \"deny\"\nstring_slice = \"deny\"\nget_unwrap = \"deny\"\narithmetic_side_effects = \"deny\"\nas_conversions = \"deny\"\ncast_possible_truncation = \"deny\"\ncast_possible_wrap = \"deny\"\ncast_sign_loss = \"deny\"\ncast_precision_loss = \"deny\"\ninteger_division = \"deny\"\ninteger_division_remainder_used = \"deny\"\nmodulo_arithmetic = \"deny\"\nfloat_arithmetic = \"deny\"\nresult_large_err = \"deny\"\nresult_unit_err = \"deny\"\nmap_err_ignore = \"deny\"\nlarge_enum_variant = \"deny\"\ncognitive_complexity = \"deny\"\ntoo_many_arguments = \"deny\"\ntoo_many_lines = \"deny\"\ntype_complexity = \"deny\"\nexcessive_nesting = \"deny\"\nawait_holding_lock = \"deny\"\nawait_holding_refcell_ref = \"deny\"\nfuture_not_send = \"deny\"\nlarge_futures = \"deny\"\ndisallowed_methods = \"deny\"\ndisallowed_macros = \"deny\"\ndisallowed_types = \"deny\"\ndisallowed_fields = \"deny\"\nmultiple_crate_versions = \"deny\"\nwildcard_dependencies = \"deny\"\nnegative_feature_names = \"deny\"\nredundant_feature_names = \"deny\"\nlet_underscore_must_use = \"deny\"\n",
    )?;
    std::fs::create_dir_all(dir.path().join(".cargo"))?;
    write_manifest(&dir, ".cargo/config.toml", "[build]\nrustflags = [\"-Z\", \"some-flag\"]\n")?;
    let exception = make_exception("BYPASS_CARGO_CONFIG_FLAGS", ".cargo/config.toml");
    let report = scan_with_exceptions(&dir, "Cargo.toml", &[exception])?;
    assert!(report.is_clean(), "expected suppression but found: {}", report.render());
    Ok(())
}

#[test]
fn non_matching_rule_does_not_suppress() -> TestResult {
    let dir = TempDir::new()?;
    write_manifest(
        &dir,
        "Cargo.toml",
        "[lints.clippy]\nunwrap_used = \"allow\"\npanic = \"deny\"\n",
    )?;
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

#[test]
fn parse_expired_exception_returns_expired_error() -> TestResult {
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
