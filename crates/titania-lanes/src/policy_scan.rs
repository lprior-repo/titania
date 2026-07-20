//! Policy-bypass scanners shared by lane binaries.
//!
//! These scanners detect local configuration that can weaken the repository's
//! strict Rust lane contract before a build command executes.

use std::path::{Path, PathBuf};

use titania_policy::Exception;

use crate::{Finding, LaneReport, RuleIdError};

pub mod cargo_config;
pub mod cargo_lints;
pub mod env_vars;
pub mod exceptions;

/// Scan policy-bypass inputs that affect a lane invocation.
///
/// The composition scans Cargo config files affecting `root`, the supplied
/// Cargo manifests for lint weakening, and the process environment for
/// compiler/toolchain override variables.
///
/// # Errors
/// Returns [`RuleIdError`] if any embedded scanner rule identifier is invalid.
pub fn scan_policy_inputs<'a>(
    root: &Path,
    manifest_paths: impl IntoIterator<Item = &'a Path>,
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    scan_policy_inputs_with_exceptions(root, manifest_paths, &[], &env_vars::real_env, report)
}

/// Scan policy-bypass inputs and suppress findings with valid strict-ai exceptions.
///
/// Exception matching is exact on the typed rule identifier and workspace path.
/// Expired or malformed exception files are handled by
/// [`exceptions::load_exceptions`] before this function is called.
///
/// The environment-variable scanner reads through the provided
/// environment reader so callers can drive the scan under a controlled
/// environment. Production callers pass the process-wide reader; tests
/// inject a `BTreeMap`-backed reader to avoid leaking host `CARGO_HOME`
/// / `RUSTUP_HOME` into the result.
///
/// # Errors
/// Returns [`RuleIdError`] if any embedded scanner rule identifier is invalid.
pub fn scan_policy_inputs_with_exceptions<'a>(
    root: &Path,
    manifest_paths: impl IntoIterator<Item = &'a Path>,
    active_exceptions: &[Exception],
    env: &env_vars::EnvReader,
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    let mut raw_report = LaneReport::new();
    cargo_config::scan_cargo_config_from(root, &mut raw_report)?;
    let normalized_manifests = manifest_paths
        .into_iter()
        .map(|manifest_path| normalize_manifest_path(root, manifest_path))
        .collect::<Vec<_>>();
    normalized_manifests.iter().try_for_each(|manifest_path| {
        cargo_lints::scan_cargo_lints_weakening(root, manifest_path, &mut raw_report).map(|_| ())
    })?;
    env_vars::scan_env_vars_with_target(&mut raw_report, env, root)?;
    report.extend_finding(filtered_findings(&raw_report, active_exceptions));
    Ok(())
}

fn normalize_manifest_path(root: &Path, manifest_path: &Path) -> PathBuf {
    manifest_path.strip_prefix(root).map_or_else(|_| manifest_path.to_path_buf(), Path::to_path_buf)
}

fn filtered_findings(report: &LaneReport, active_exceptions: &[Exception]) -> Vec<Finding> {
    report
        .findings()
        .iter()
        .filter(|finding| !exceptions::finding_is_excepted(finding, active_exceptions))
        .cloned()
        .collect()
}
