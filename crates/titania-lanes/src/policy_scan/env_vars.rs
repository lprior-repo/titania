use std::path::{Path, PathBuf};

use crate::{Finding, LaneReport, RuleId, RuleIdError};

const RULE_RUSTFLAGS: &str = "BYPASS_ENV_RUSTFLAGS";
const RULE_CARGO_ENCODED_RUSTFLAGS: &str = "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS";
const RULE_RUSTC_WRAPPER: &str = "BYPASS_ENV_RUSTC_WRAPPER";
const RULE_RUSTC_WORKSPACE_WRAPPER: &str = "BYPASS_ENV_RUSTC_WORKSPACE_WRAPPER";
const RULE_RUSTC_BOOTSTRAP: &str = "BYPASS_ENV_RUSTC_BOOTSTRAP";
const RULE_CARGO_HOME: &str = "BYPASS_ENV_CARGO_HOME";
const RULE_RUSTUP_HOME: &str = "BYPASS_ENV_RUSTUP_HOME";
const FLAGS_MESSAGE: &str = "overrides cargo flags and bypasses lane discipline";
const WRAPPER_MESSAGE: &str =
    "must be sccache or absent; other wrappers replace rustc and bypass lane checks";
const WORKSPACE_WRAPPER_MESSAGE: &str =
    "must be sccache or absent; other wrappers replace rustc for the workspace";
const BOOTSTRAP_MESSAGE: &str = "enables unstable features and bypasses stability gates";

pub const CONTROLLED_CARGO_HOME_SUFFIX: &str = "cargo-home";
pub const CONTROLLED_RUSTUP_HOME_SUFFIX: &str = "rustup-home";

pub type EnvReader = dyn Fn(&str) -> Option<String>;

#[derive(Debug, Clone, Copy)]
enum EnvValuePolicy {
    AnyNonEmptyForbidden,
    WrapperMustBeSccache,
    PresenceForbidden,
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
    fn into_rule(self) -> Result<EnvRule, RuleIdError> {
        Ok(EnvRule {
            name: self.name,
            rule: RuleId::new(self.rule)?,
            message: self.message,
            policy: self.policy,
        })
    }
}

pub fn scan_env_vars(report: &mut LaneReport) -> Result<(), RuleIdError> {
    scan_env_vars_with(report, &real_env)
}

pub fn scan_env_vars_with(report: &mut LaneReport, env: &EnvReader) -> Result<(), RuleIdError> {
    env_rules()?.iter().for_each(|rule| check_env_var(rule, report, env));
    Ok(())
}

pub fn scan_env_vars_with_target(
    report: &mut LaneReport,
    env: &EnvReader,
    root: &Path,
) -> Result<(), RuleIdError> {
    scan_env_vars_with(report, env)?;
    check_controlled_home(cargo_home_spec(), report, env, root)?;
    check_controlled_home(rustup_home_spec(), report, env, root)
}

#[must_use]
pub fn controlled_home_path(root: &Path, suffix: &str) -> PathBuf {
    root.join(".titania").join("hermetic").join(suffix)
}

#[derive(Debug, Clone, Copy)]
struct ControlledHomeSpec {
    name: &'static str,
    rule_id: &'static str,
    suffix: &'static str,
}

const fn cargo_home_spec() -> ControlledHomeSpec {
    ControlledHomeSpec {
        name: "CARGO_HOME",
        rule_id: RULE_CARGO_HOME,
        suffix: CONTROLLED_CARGO_HOME_SUFFIX,
    }
}

const fn rustup_home_spec() -> ControlledHomeSpec {
    ControlledHomeSpec {
        name: "RUSTUP_HOME",
        rule_id: RULE_RUSTUP_HOME,
        suffix: CONTROLLED_RUSTUP_HOME_SUFFIX,
    }
}

fn controlled_home_message(name: &str, controlled: &Path, supplied: Option<&str>) -> String {
    match supplied {
        None => format!("{name} must be set to controlled path {}", controlled.display()),
        Some("") => {
            format!("{name} must be non-empty and equal controlled path {}", controlled.display())
        }
        Some(value) => {
            format!("{name} must equal controlled path {} (got {value})", controlled.display())
        }
    }
}

fn check_controlled_home(
    spec: ControlledHomeSpec,
    report: &mut LaneReport,
    env: &EnvReader,
    root: &Path,
) -> Result<(), RuleIdError> {
    let ControlledHomeSpec { name, rule_id, suffix } = spec;
    let controlled = controlled_home_path(root, suffix);
    let supplied = env(name);
    let matches_controlled = supplied
        .as_deref()
        .is_some_and(|value| !value.is_empty() && Path::new(value) == controlled.as_path());
    if matches_controlled {
        return Ok(());
    }
    let message = controlled_home_message(name, &controlled, supplied.as_deref());
    report.push(Finding::new(RuleId::new(rule_id)?, "env", 0, message));
    Ok(())
}

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
    presence_rule("RUSTC_BOOTSTRAP", RULE_RUSTC_BOOTSTRAP, BOOTSTRAP_MESSAGE)
}

const fn forbid_rule(name: &'static str, rule: &'static str, message: &'static str) -> EnvRuleSpec {
    env_rule(name, rule, message, EnvValuePolicy::AnyNonEmptyForbidden)
}

const fn presence_rule(
    name: &'static str,
    rule: &'static str,
    message: &'static str,
) -> EnvRuleSpec {
    env_rule(name, rule, message, EnvValuePolicy::PresenceForbidden)
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
        EnvValuePolicy::PresenceForbidden => true,
        EnvValuePolicy::WrapperMustBeSccache => !value.is_empty() && !is_sccache_wrapper(value),
    }
}

fn is_sccache_wrapper(value: &str) -> bool {
    value == "sccache"
        || Path::new(value).file_name().and_then(|file_name| file_name.to_str()) == Some("sccache")
}

pub(crate) fn real_env(name: &str) -> Option<String> {
    let Ok(value) = std::env::var(name) else {
        return None;
    };
    Some(value)
}
