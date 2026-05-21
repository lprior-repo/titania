//! Proof/test lane definitions and command generation.
//!
//! Each lane maps to a shell command with crate-specific arguments.

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Lane {
    pub name: String,
    pub required: bool,
}

pub fn lane_command(lane: &Lane, crate_name: &str, workspace_root: &Path) -> Vec<String> {
    match lane.name.as_str() {
        "test" => vec![
            "cargo".into(),
            "test".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "clippy" => vec![
            "cargo".into(),
            "clippy".into(),
            "-p".into(),
            crate_name.into(),
            "--".into(),
            "-D".into(),
            "warnings".into(),
        ],
        "nextest" => vec![
            "cargo".into(),
            "nextest".into(),
            "run".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "kani" => vec![
            "cargo".into(),
            "kani".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "miri" => vec![
            "cargo".into(),
            "+nightly".into(),
            "miri".into(),
            "test".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "loom" => vec![
            "cargo".into(),
            "test".into(),
            "-p".into(),
            crate_name.into(),
            "--features".into(),
            "loom".into(),
        ],
        "fuzz" => vec![
            "cargo".into(),
            "fuzz".into(),
            "run".into(),
            format!("{crate_name}_fuzz"),
        ],
        "mutants" => vec![
            "cargo".into(),
            "mutants".into(),
            "-p".into(),
            crate_name.into(),
            "--no-times".into(),
        ],
        "coverage" => vec![
            "cargo".into(),
            "llvm-cov".into(),
            "--no-report".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "verus" => verus_command(crate_name, workspace_root),
        "tla" => tla_command(crate_name, workspace_root),
        "flux" => vec![
            "cargo".into(),
            "flux".into(),
            "-p".into(),
            crate_name.into(),
        ],
        _ => vec!["echo".into(), format!("unknown lane: {}", lane.name)],
    }
}

fn verus_command(crate_name: &str, workspace_root: &Path) -> Vec<String> {
    let verus_dir = workspace_root.join("verification").join("verus");
    vec![
        "verus".into(),
        format!("{}/{crate_name}.rs", verus_dir.display()),
    ]
}

fn tla_command(crate_name: &str, workspace_root: &Path) -> Vec<String> {
    let tla_file = workspace_root
        .join("verification")
        .join("tla")
        .join(format!("{crate_name}.tla"));
    vec!["tla2tools".into(), format!("{}", tla_file.display())]
}

pub fn detect_available_lanes(workspace_root: &Path) -> Vec<Lane> {
    let all_lanes = [
        ("test", true),
        ("clippy", true),
        ("nextest", false),
        ("kani", false),
        ("miri", false),
        ("loom", false),
        ("fuzz", false),
        ("mutants", false),
        ("coverage", false),
        ("verus", false),
        ("tla", false),
        ("flux", false),
    ];

    all_lanes
        .iter()
        .filter(|(name, required)| {
            if *required {
                true
            } else {
                is_tool_available(name, workspace_root)
            }
        })
        .map(|(name, required)| Lane {
            name: name.to_string(),
            required: *required,
        })
        .collect()
}

fn is_tool_available(lane: &str, workspace_root: &Path) -> bool {
    match lane {
        "nextest" => tool_in_path("cargo-nextest"),
        "kani" => tool_in_path("cargo-kani"),
        "miri" => tool_in_path("cargo-miri"),
        "loom" => has_crate_feature("loom", workspace_root),
        "fuzz" => tool_in_path("cargo-fuzz"),
        "mutants" => tool_in_path("cargo-mutants"),
        "coverage" => tool_in_path("cargo-llvm-cov"),
        "verus" => workspace_root.join("verification/verus").exists(),
        "tla" => workspace_root.join("verification/tla").exists(),
        "flux" => tool_in_path("cargo-flux"),
        _ => false,
    }
}

fn tool_in_path(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_crate_feature(_feature: &str, _workspace_root: &Path) -> bool {
    // Simplified: check if any crate has the feature
    // In production, parse Cargo.toml for feature flags
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lane_command_test() {
        let lane = Lane {
            name: "test".to_string(),
            required: true,
        };
        let cmd = lane_command(&lane, "vb_core", Path::new("/workspace"));
        assert_eq!(cmd[0], "cargo");
        assert_eq!(cmd[1], "test");
    }

    #[test]
    fn test_lane_command_clippy() {
        let lane = Lane {
            name: "clippy".to_string(),
            required: true,
        };
        let cmd = lane_command(&lane, "vb_core", Path::new("/workspace"));
        assert!(cmd.contains(&"-D".to_string()));
        assert!(cmd.contains(&"warnings".to_string()));
    }

    #[test]
    fn test_required_lanes_always_available() {
        let lanes = detect_available_lanes(Path::new("/workspace"));
        let required: Vec<_> = lanes.iter().filter(|l| l.required).collect();
        assert!(!required.is_empty());
    }
}
