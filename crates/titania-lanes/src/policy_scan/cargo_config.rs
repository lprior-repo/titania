//! Cargo configuration policy scanner.
//!
//! Detects `.cargo/config` and `.cargo/config.toml` entries that can bypass
//! the lane build discipline. `rustflags` values are always violations;
//! `rustc-wrapper` and `workspace-wrapper` must be absent or `sccache`.

use std::path::Path;

use toml_edit::{DocumentMut, Item, Value};

use crate::{Finding, LaneReport, RuleId, RuleIdError};

const RULE_FLAGS: &str = "BYPASS_CARGO_CONFIG_FLAGS";
const RULE_WRAPPER: &str = "BYPASS_CARGO_CONFIG_WRAPPER";
const CONFIG_NAMES: &[&str] = &["config", "config.toml"];
const RULE_PARENT: &str = "BYPASS_CARGO_CONFIG_PARENT";

#[derive(Debug, Clone)]
struct CargoConfigRules {
    flags: RuleId,
    wrapper: RuleId,
    parent: RuleId,
}

impl CargoConfigRules {
    /// Construct validated Cargo-config rule identifiers.
    ///
    /// # Errors
    /// Returns [`RuleIdError`] if an embedded rule literal is invalid.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            flags: RuleId::new(RULE_FLAGS)?,
            wrapper: RuleId::new(RULE_WRAPPER)?,
            parent: RuleId::new(RULE_PARENT)?,
        })
    }
}

/// Scan Cargo configuration files affecting the process current directory.
///
/// # Errors
/// Returns [`RuleIdError`] if an embedded rule identifier is invalid.
pub fn scan_cargo_config(report: &mut LaneReport) -> Result<(), RuleIdError> {
    std::env::current_dir().map_or(Ok(()), |cwd| scan_cargo_config_from(&cwd, report))
}

/// Scan Cargo configuration files from the resolved target root.
///
/// The target root's own `.cargo/config` and `.cargo/config.toml` files are
/// checked for forbidden values. Cargo config files in ancestor directories are
/// rejected outright because Cargo would apply them outside the checked-in
/// target project policy boundary.
///
/// # Errors
/// Returns [`RuleIdError`] if an embedded rule identifier is invalid.
pub fn scan_cargo_config_from(start: &Path, report: &mut LaneReport) -> Result<(), RuleIdError> {
    let rules = CargoConfigRules::new()?;
    let findings = scan_target_config(start, &rules)
        .into_iter()
        .chain(parent_config_findings(start, &rules))
        .collect::<Vec<_>>();
    report.extend_finding(findings);
    Ok(())
}

fn scan_target_config(root: &Path, rules: &CargoConfigRules) -> Vec<Finding> {
    CONFIG_NAMES
        .iter()
        .flat_map(|name| scan_config_path(root, &root.join(".cargo").join(name), rules))
        .collect()
}

fn parent_config_findings(root: &Path, rules: &CargoConfigRules) -> Vec<Finding> {
    root.ancestors()
        .skip(1)
        .flat_map(|dir| parent_config_findings_in_dir(root, dir, rules))
        .collect()
}

fn parent_config_findings_in_dir(
    root: &Path,
    dir: &Path,
    rules: &CargoConfigRules,
) -> Vec<Finding> {
    CONFIG_NAMES
        .iter()
        .filter_map(|name| parent_config_finding(root, &dir.join(".cargo").join(name), rules))
        .collect()
}

fn parent_config_finding(root: &Path, path: &Path, rules: &CargoConfigRules) -> Option<Finding> {
    path.is_file().then(|| {
        Finding::new(
            rules.parent.clone(),
            relative_path(root, path),
            0,
            "parent-directory Cargo config is rejected; keep lane policy config inside the target project",
        )
    })
}

fn scan_config_path(base: &Path, path: &Path, rules: &CargoConfigRules) -> Vec<Finding> {
    if !path.is_file() {
        return Vec::new();
    }
    std::fs::read_to_string(path).map_or_else(
        |_| Vec::new(),
        |content| document_findings(parse_config(&content), &relative_path(base, path), rules),
    )
}

fn parse_config(content: &str) -> Option<DocumentMut> {
    content.parse::<DocumentMut>().ok()
}

fn document_findings(
    document: Option<DocumentMut>,
    path: &str,
    rules: &CargoConfigRules,
) -> Vec<Finding> {
    document
        .and_then(|document| build_item(&document).cloned())
        .map_or_else(Vec::new, |build| build_findings(&build, path, rules))
}

fn build_item(document: &DocumentMut) -> Option<&Item> {
    document.get("build")
}

fn build_findings(build: &Item, path: &str, rules: &CargoConfigRules) -> Vec<Finding> {
    let wrappers = [
        wrapper_finding(wrapper_value(build, "rustc-wrapper"), path, rules),
        wrapper_finding(wrapper_value(build, "workspace-wrapper"), path, rules),
    ];
    wrappers
        .into_iter()
        .flatten()
        .chain(rustflags_values(build).into_iter().map(|flag| rustflag_finding(&flag, path, rules)))
        .collect()
}

fn wrapper_value<'a>(build: &'a Item, key: &str) -> Option<&'a str> {
    build
        .as_table()
        .and_then(|table| table.get(key))
        .and_then(Item::as_value)
        .and_then(Value::as_str)
}

fn rustflags_values(build: &Item) -> Vec<String> {
    build
        .as_table()
        .and_then(|table| table.get("rustflags"))
        .and_then(Item::as_value)
        .map(rustflags_value_to_strings)
        .into_iter()
        .flatten()
        .collect()
}

fn rustflags_value_to_strings(value: &Value) -> Vec<String> {
    if let Some(flag) = value.as_str() {
        return vec![flag.to_owned()];
    }
    if let Some(array) = value.as_array() {
        return array.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect();
    }
    vec![value.to_string()]
}

fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base).map_or_else(
        |_| path.to_string_lossy().into_owned(),
        |relative| relative.to_string_lossy().into_owned(),
    )
}

fn wrapper_finding(value: Option<&str>, path: &str, rules: &CargoConfigRules) -> Option<Finding> {
    match value {
        Some(wrapper) if wrapper != "sccache" => Some(Finding::new(
            rules.wrapper.clone(),
            path,
            0,
            format!("non-standard rustc wrapper {wrapper:?} - must be absent or sccache"),
        )),
        Some(_) | None => None,
    }
}

fn rustflag_finding(flag: &str, path: &str, rules: &CargoConfigRules) -> Finding {
    Finding::new(
        rules.flags.clone(),
        path,
        0,
        format!(
            "unexpected rustflag {flag:?} - policy flags belong in Cargo.toml, not .cargo/config"
        ),
    )
}
