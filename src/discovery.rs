//! Workspace crate discovery via cargo metadata.
//!
//! Calls `cargo metadata --no-deps --format-version 1` exactly once
//! and parses the output into `CrateInfo` records.

use cargo_metadata::{Metadata, MetadataCommand, Package};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<String>,
}

pub fn discover_crates(workspace_root: &std::path::Path) -> anyhow::Result<Vec<CrateInfo>> {
    let metadata = run_cargo_metadata(workspace_root)?;
    Ok(parse_crates(&metadata))
}

fn run_cargo_metadata(workspace_root: &std::path::Path) -> anyhow::Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .map_err(|e| anyhow::anyhow!("cargo metadata failed: {e}"))
}

fn parse_crates(metadata: &Metadata) -> Vec<CrateInfo> {
    let workspace_members: std::collections::HashSet<_> = metadata
        .workspace_members
        .iter()
        .map(|id| id.repr.as_str())
        .collect();

    metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(pkg.id.repr.as_str()))
        .filter(|pkg| pkg.name != "xtask")
        .filter(|pkg| pkg.name != "workspace_tests")
        .filter(|pkg| pkg.name != "vb_benchmark")
        .map(pkg_to_crate_info)
        .collect()
}

fn pkg_to_crate_info(pkg: &Package) -> CrateInfo {
    let deps: Vec<String> = pkg
        .dependencies
        .iter()
        .filter(|d| d.path.is_some())
        .map(|d| d.name.clone())
        .collect();

    CrateInfo {
        name: pkg.name.clone(),
        manifest_path: pkg.manifest_path.clone().into(),
        dependencies: deps,
    }
}

pub fn filter_crates(
    crates: &[CrateInfo],
    include: Option<&[String]>,
    exclude: Option<&[String]>,
) -> Vec<CrateInfo> {
    crates
        .iter()
        .filter(|c| match include {
            Some(patterns) => matches_any(&c.name, patterns),
            None => true,
        })
        .filter(|c| match exclude {
            Some(patterns) => !matches_any(&c.name, patterns),
            None => true,
        })
        .cloned()
        .collect()
}

fn matches_any(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| name.contains(p.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_crates_include() {
        let crates = make_test_crates(&["vb_core", "vb_cli", "vb_storage"]);
        let filtered = filter_crates(&crates, Some(&["vb_core".to_string()]), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "vb_core");
    }

    #[test]
    fn test_filter_crates_exclude() {
        let crates = make_test_crates(&["vb_core", "vb_cli", "vb_storage"]);
        let filtered = filter_crates(&crates, None, Some(&["vb_cli".to_string()]));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_crates_include_and_exclude() {
        let crates = make_test_crates(&["vb_core", "vb_cli", "vb_storage"]);
        let filtered = filter_crates(
            &crates,
            Some(&["vb_".to_string()]),
            Some(&["vb_cli".to_string()]),
        );
        assert_eq!(filtered.len(), 2);
    }

    fn make_test_crates(names: &[&str]) -> Vec<CrateInfo> {
        names
            .iter()
            .map(|n| CrateInfo {
                name: n.to_string(),
                manifest_path: PathBuf::from(format!("crates/{n}/Cargo.toml")),
                dependencies: vec![],
            })
            .collect()
    }
}
