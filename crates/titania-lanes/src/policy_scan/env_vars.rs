//! Environment-variable policy scanner.
//!
//! Detects ambient variables that can override the lane toolchain or compiler
//! flags. Unset and empty variables are clean.

use std::path::Path;

use crate::{Finding, LaneReport, RuleId, RuleIdError};

const RULE_RUSTFLAGS: &str = "BYPASS_ENV_RUSTFLAGS";
const RULE_CARGO_ENCODED_RUSTFLAGS: &str = "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS";
const RULE_RUSTC_WRAPPER: &str = "BYPASS_ENV_RUSTC_WRAPPER";
const RULE_RUSTC_WORKSPACE_WRAPPER: &str = "BYPASS_ENV_RUSTC_WORKSPACE_WRAPPER";
const RULE_RUSTC_BOOTSTRAP: &str = "BYPASS_ENV_RUSTC_BOOTSTRAP";
const FLAGS_MESSAGE: &str = "overrides cargo flags and bypasses lane discipline";
const WRAPPER_MESSAGE: &str =
    "must be sccache or absent; other wrappers replace rustc and bypass lane checks";
const WORKSPACE_WRAPPER_MESSAGE: &str =
    "must be sccache or absent; other wrappers replace rustc for the workspace";
const BOOTSTRAP_MESSAGE: &str = "enables unstable features and bypasses stability gates";

/// Reader used to obtain a named environment variable.
pub type EnvReader = dyn Fn(&str) -> Option<String>;

#[derive(Debug, Clone, Copy)]
enum EnvValuePolicy {
    AnyNonEmptyForbidden,
    WrapperMustBeSccache,
}

struct EnvRule {
    name: &'static str,
    rule: RuleId,
    message: &'static str,
    policy: EnvValuePolicy,
}

#[derive(Debug, Clone, Copy)]
struct EnvRuleSpec {
    name: &'static str,
    rule: &'static str,
    message: &'static str,
    policy: EnvValuePolicy,
}

impl EnvRuleSpec {
    /// Convert a static rule spec into a validated environment rule.
    ///
    /// # Errors
    /// Returns [`RuleIdError`] if the embedded rule literal is invalid.
    fn into_rule(self) -> Result<EnvRule, RuleIdError> {
        Ok(EnvRule {
            name: self.name,
            rule: RuleId::new(self.rule)?,
            message: self.message,
            policy: self.policy,
        })
    }
}

/// Scan the process environment for forbidden lane-bypass variables.
///
/// # Errors
/// Returns [`RuleIdError`] if an embedded rule identifier is invalid.
pub fn scan_env_vars(report: &mut LaneReport) -> Result<(), RuleIdError> {
    scan_env_vars_with(report, &real_env)
}

/// Scan a supplied environment source for forbidden lane-bypass variables.
///
/// This is public so behavior tests and deterministic callers can avoid
/// mutating the process environment.
///
/// # Errors
/// Returns [`RuleIdError`] if an embedded rule identifier is invalid.
pub fn scan_env_vars_with(report: &mut LaneReport, env: &EnvReader) -> Result<(), RuleIdError> {
    env_rules()?.iter().for_each(|rule| check_env_var(rule, report, env));
    Ok(())
}

/// Construct validated environment-rule descriptors.
///
/// # Errors
/// Returns [`RuleIdError`] if an embedded rule literal is invalid.
fn env_rules() -> Result<[EnvRule; 5], RuleIdError> {
    Ok([
        rustflags_rule().into_rule()?,
        encoded_rustflags_rule().into_rule()?,
        rustc_wrapper_rule().into_rule()?,
        workspace_wrapper_rule().into_rule()?,
        bootstrap_rule().into_rule()?,
    ])
}

const fn rustflags_rule() -> EnvRuleSpec {
    forbid_rule("RUSTFLAGS", RULE_RUSTFLAGS, FLAGS_MESSAGE)
}

const fn encoded_rustflags_rule() -> EnvRuleSpec {
    forbid_rule("CARGO_ENCODED_RUSTFLAGS", RULE_CARGO_ENCODED_RUSTFLAGS, FLAGS_MESSAGE)
}

const fn rustc_wrapper_rule() -> EnvRuleSpec {
    wrapper_rule("RUSTC_WRAPPER", RULE_RUSTC_WRAPPER, WRAPPER_MESSAGE)
}

const fn workspace_wrapper_rule() -> EnvRuleSpec {
    wrapper_rule("RUSTC_WORKSPACE_WRAPPER", RULE_RUSTC_WORKSPACE_WRAPPER, WORKSPACE_WRAPPER_MESSAGE)
}

const fn bootstrap_rule() -> EnvRuleSpec {
    forbid_rule("RUSTC_BOOTSTRAP", RULE_RUSTC_BOOTSTRAP, BOOTSTRAP_MESSAGE)
}

const fn forbid_rule(name: &'static str, rule: &'static str, message: &'static str) -> EnvRuleSpec {
    env_rule(name, rule, message, EnvValuePolicy::AnyNonEmptyForbidden)
}

const fn wrapper_rule(
    name: &'static str,
    rule: &'static str,
    message: &'static str,
) -> EnvRuleSpec {
    env_rule(name, rule, message, EnvValuePolicy::WrapperMustBeSccache)
}

const fn env_rule(
    name: &'static str,
    rule: &'static str,
    message: &'static str,
    policy: EnvValuePolicy,
) -> EnvRuleSpec {
    EnvRuleSpec { name, rule, message, policy }
}

fn check_env_var(rule: &EnvRule, report: &mut LaneReport, env: &EnvReader) {
    match env(rule.name) {
        Some(value) if env_value_violates(&value, rule.policy) => report.push(Finding::new(
            rule.rule.clone(),
            "env",
            0,
            format!("{} is set - {}", rule.name, rule.message),
        )),
        Some(_) | None => {}
    }
}

fn env_value_violates(value: &str, policy: EnvValuePolicy) -> bool {
    match policy {
        EnvValuePolicy::AnyNonEmptyForbidden => !value.is_empty(),
        EnvValuePolicy::WrapperMustBeSccache => !value.is_empty() && !is_sccache_wrapper(value),
    }
}

fn is_sccache_wrapper(value: &str) -> bool {
    value == "sccache"
        || Path::new(value).file_name().and_then(|file_name| file_name.to_str()) == Some("sccache")
}

fn real_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}
