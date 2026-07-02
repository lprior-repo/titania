use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use titania_lanes::{Finding, LaneReport};

use super::{
    FORBIDDEN_FEATURE_NAMES, WsRules,
    toml_scan::{binary_names, named_table_values, package_name, quoted_array_values},
};

fn expected_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(std::string::ToString::to_string).collect()
}

pub(super) fn check_workspace_members(root: &Path, rules: &WsRules, report: &mut LaneReport) {
    let cargo_path = root.join("Cargo.toml");
    let manifest = match fs::read_to_string(&cargo_path) {
        Ok(text) => text,
        Err(error) => {
            report.push(unreadable_manifest_finding(rules, &error));
            return;
        }
    };
    let actual = quoted_array_values(&manifest, "members");
    report.record_scan();
    if actual.is_empty() {
        report.push(missing_members_finding(rules));
    } else if let Err(error) = super::write_stderr_line(format_args!(
        "[check-workspace-assertions] workspace members: {actual:?}"
    )) {
        report.push(stderr_output_finding(rules, &error));
    }
}

fn unreadable_manifest_finding(rules: &WsRules, error: &std::io::Error) -> Finding {
    Finding::new(
        rules.unreadable.clone(),
        "Cargo.toml",
        0,
        format!("Cargo.toml: unreadable: {error}"),
    )
}

fn missing_members_finding(rules: &WsRules) -> Finding {
    Finding::new(
        rules.members.clone(),
        "Cargo.toml",
        0,
        "Cargo.toml: workspace.members is empty or missing",
    )
}

fn stderr_output_finding(rules: &WsRules, error: &std::io::Error) -> Finding {
    Finding::new(rules.unreadable.clone(), "stderr", 0, format!("stderr write failed: {error}"))
}

pub(super) fn check_crate_names(
    root: &Path,
    members: &[String],
    rules: &WsRules,
    report: &mut LaneReport,
) {
    let findings: Vec<Finding> = members
        .iter()
        .inspect(|_| report.record_scan())
        .flat_map(|member| check_crate_name(root, member, rules))
        .collect();
    report.extend_finding(findings);
}

fn check_crate_name(root: &Path, member: &str, rules: &WsRules) -> Vec<Finding> {
    let manifest_path = root.join(member).join("Cargo.toml");
    let Ok(manifest) = fs::read_to_string(&manifest_path) else {
        return vec![Finding::new(
            rules.unreadable.clone(),
            format!("{member}/Cargo.toml"),
            0,
            format!("{member}/Cargo.toml: unreadable manifest"),
        )];
    };
    let mut findings = check_package_name(member, &manifest, rules);
    if let Err(error) = report_lanes_bins(member, &manifest) {
        findings.push(Finding::new(
            rules.unreadable.clone(),
            format!("{member}/Cargo.toml"),
            0,
            format!("stderr write failed: {error}"),
        ));
    }
    findings.extend(check_forbidden_features(member, &manifest, rules));
    findings
}

fn check_package_name(member: &str, manifest: &str, rules: &WsRules) -> Vec<Finding> {
    if package_name(manifest).is_none() {
        vec![Finding::new(
            rules.crate_name.clone(),
            format!("{member}/Cargo.toml"),
            0,
            format!("{member}/Cargo.toml: missing or malformed `name =`"),
        )]
    } else {
        Vec::new()
    }
}

/// Report binary targets for the lanes crate when present.
///
/// # Errors
///
/// Returns the underlying stderr write error if the diagnostic line cannot be
/// emitted.
fn report_lanes_bins(member: &str, manifest: &str) -> Result<(), std::io::Error> {
    let bins = binary_names(manifest);
    if member.ends_with("titania-lanes") && !bins.is_empty() {
        return super::write_stderr_line(format_args!(
            "[check-workspace-assertions] {member} bins: {bins:?}"
        ));
    }
    Ok(())
}

fn check_forbidden_features(member: &str, manifest: &str, rules: &WsRules) -> Vec<Finding> {
    let features = named_table_values(manifest, "[features]");
    let forbidden: Vec<String> =
        features.intersection(&expected_set(FORBIDDEN_FEATURE_NAMES)).cloned().collect();
    if forbidden.is_empty() {
        Vec::new()
    } else {
        vec![Finding::new(
            rules.forbidden_feature.clone(),
            format!("{member}/Cargo.toml"),
            0,
            format!("{member}/Cargo.toml: forbidden feature names {forbidden:?}"),
        )]
    }
}

pub(super) fn check_forbidden_dependencies(
    root: &Path,
    members: &[String],
    rules: &WsRules,
    report: &mut LaneReport,
) {
    let forbidden = expected_set(FORBIDDEN_FEATURE_NAMES);
    let findings: Vec<Finding> = members
        .iter()
        .filter_map(|member| check_forbidden_dependency(root, member, &forbidden, rules))
        .collect();
    report.extend_finding(findings);
}

fn check_forbidden_dependency(
    root: &Path,
    member: &str,
    forbidden: &BTreeSet<String>,
    rules: &WsRules,
) -> Option<Finding> {
    let manifest_path = root.join(member).join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path).ok()?;
    let deps = dependency_names(&manifest);
    let hits: Vec<String> = deps.intersection(forbidden).cloned().collect();
    forbidden_dependency_finding(member, &hits, rules)
}

fn forbidden_dependency_finding(member: &str, hits: &[String], rules: &WsRules) -> Option<Finding> {
    if hits.is_empty() {
        return None;
    }
    Some(Finding::new(
        rules.forbidden_dep.clone(),
        format!("{member}/Cargo.toml"),
        0,
        format!("{member}/Cargo.toml: forbidden dependency {hits:?}"),
    ))
}

fn dependency_names(manifest: &str) -> BTreeSet<String> {
    ["[dependencies]", "[dev-dependencies]", "[build-dependencies]"]
        .into_iter()
        .flat_map(|table| named_table_values(manifest, table).into_iter())
        .collect()
}

pub(super) fn check_generated_boundaries(root: &Path, rules: &WsRules, report: &mut LaneReport) {
    let findings: Vec<Finding> = collect_generated_dirs(root)
        .into_iter()
        .flat_map(|dir| rust_files(&dir).into_iter())
        .flat_map(|source| check_generated_file(root, &source, rules))
        .collect();
    report.extend_finding(findings);
}

fn check_generated_file(root: &Path, source: &Path, rules: &WsRules) -> Vec<Finding> {
    let Ok(text) = fs::read_to_string(source) else {
        return Vec::new();
    };
    FORBIDDEN_FEATURE_NAMES
        .iter()
        .filter(|forbidden| text.contains(**forbidden))
        .map(|forbidden| {
            let rel = source
                .strip_prefix(root)
                .map_or_else(|_| source.display().to_string(), |path| path.display().to_string());
            Finding::new(
                rules.generated_boundary.clone(),
                rel,
                0,
                format!("forbidden generated-boundary token: {forbidden}"),
            )
        })
        .collect()
}

fn collect_generated_dirs(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root.join("crates")).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .flatten()
                .map(|entry| entry.path().join("src").join("generated"))
                .filter(|path| path.exists())
                .collect()
        },
    )
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries.flatten().flat_map(|entry| rust_file_entry(entry.path()).into_iter()).collect()
        },
    )
}

fn rust_file_entry(path: PathBuf) -> Vec<PathBuf> {
    if path.is_dir() {
        rust_files(&path)
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        vec![path]
    } else {
        Vec::new()
    }
}

pub(super) fn discover_members(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("Cargo.toml")).map_or_else(
        |_| Vec::new(),
        |manifest| quoted_array_values(&manifest, "members").into_iter().collect(),
    )
}
