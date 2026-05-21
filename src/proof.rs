//! Proof system commands for velvet-ballastics.
//!
//! Provides:
//! - `proof-plan --changed` - maps changed files to proof obligations
//! - `proof-check --changed` - runs required proof checks
//! - `proof-check --level L3` - runs only specific proof level
//! - `proof-evidence --bead <id>` - writes evidence bundle
//! - `proof-drift` - checks spec/code alignment

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofObligation {
    pub id: String,
    pub section: Vec<usize>,
    pub statement: String,
    pub proof_level: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub files: Vec<String>,
    pub required: RequiredProof,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredProof {
    #[serde(default)]
    pub verus: Option<String>,
    #[serde(default)]
    pub kani: Option<String>,
    #[serde(default)]
    pub tests: Vec<String>,
    #[serde(default)]
    pub fuzz: Option<FuzzField>,
    #[serde(default)]
    pub tla: Option<String>,
    #[serde(default)]
    pub loom: Option<String>,
    #[serde(default)]
    pub property_tests: Vec<String>,
    #[serde(default)]
    pub differential_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FuzzField {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProofLevel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProofMatrix {
    pub crate_name: String,
    pub proof_strategy: String,
    pub primary_level: String,
    pub secondary_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProofTarget {
    pub obligation_id: String,
    pub file: PathBuf,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObligationStatus {
    pub id: String,
    pub status: String,
    pub level: String,
    pub commands: Vec<String>,
    pub log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidence {
    pub kind: String,
    pub bead: String,
    pub commit: String,
    pub obligations: Vec<ObligationStatus>,
    pub remaining_assumptions: Vec<String>,
    pub verified: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ProofObligationsFile {
    obligations: Vec<ProofObligation>,
}

pub fn load_proof_obligations() -> Result<Vec<ProofObligation>, String> {
    let path = PathBuf::from("contracts/proof_obligations.yaml");
    if !path.exists() {
        return Err(format!(
            "Proof obligations file not found: {}",
            path.display()
        ));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: ProofObligationsFile = serde_saphyr::from_str(&content).map_err(|e| e.to_string())?;
    Ok(file.obligations)
}

#[allow(dead_code)]
pub fn obligations_for_files(
    obligations: &[ProofObligation],
    changed_files: &[&str],
) -> Vec<ProofObligation> {
    let changed_set: std::collections::HashSet<_> = changed_files.iter().collect();
    obligations
        .iter()
        .filter(|obl| obl.files.iter().any(|f| changed_set.contains(&f.as_str())))
        .cloned()
        .collect()
}

pub fn obligations_for_level(obligations: &[ProofObligation], level: &str) -> Vec<ProofObligation> {
    obligations
        .iter()
        .filter(|obl| obl.proof_level == level)
        .cloned()
        .collect()
}

pub fn commands_for_obligation(obl: &ProofObligation) -> Vec<String> {
    let mut cmds = Vec::new();

    if let Some(ref kani) = obl.required.kani {
        cmds.push(format!("cargo kani --harness {}", kani));
    }
    if let Some(ref verus) = obl.required.verus {
        cmds.push(format!("verus {}", verus));
    }
    for test in &obl.required.tests {
        cmds.push(format!("cargo nextest run -p {} {}", obl.crate_name, test));
    }
    if let Some(ref fuzz) = obl.required.fuzz {
        match fuzz {
            FuzzField::Single(s) => cmds.push(format!("cargo fuzz run {}", s)),
            FuzzField::Multiple(v) => {
                for f in v {
                    cmds.push(format!("cargo fuzz run {}", f));
                }
            }
        }
    }
    if let Some(ref tla) = obl.required.tla {
        cmds.push(format!("tla2tools {}", tla));
    }
    if let Some(ref loom) = obl.required.loom {
        cmds.push(format!("cargo xtask loom --model {}", loom));
    }

    cmds
}

#[allow(dead_code)]
pub fn proof_levels() -> Vec<(&'static str, &'static str)> {
    vec![
        ("L0", "Mechanical scan / lint / CI evidence"),
        ("L1", "Unit + property + differential tests"),
        ("L2", "Fuzz / mutation / crash-lab evidence"),
        ("L3", "Bounded model checking: Kani / Loom"),
        ("L4", "Deductive Rust proof: Verus / Creusot"),
        ("L5", "Proof assistant model: Lean via Aeneas"),
        ("L6", "Operational evidence: benchmark, replay, recovery"),
    ]
}

pub fn write_proof_evidence(
    bead_id: &str,
    obligations: &[ProofObligation],
    results: &[(String, bool)],
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let obligations_status: Vec<ObligationStatus> = results
        .iter()
        .map(|(id, passed)| {
            let Some(obl) = obligations.iter().find(|o| o.id == *id) else {
                return Err(format!("Obligation not found: {id}"));
            };
            Ok(ObligationStatus {
                id: id.clone(),
                status: if *passed {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                level: obl.proof_level.clone(),
                commands: commands_for_obligation(obl),
                log: None,
            })
        })
        .collect::<Result<Vec<ObligationStatus>, String>>()?;

    let evidence = ProofEvidence {
        kind: "ProofEvidence".to_string(),
        bead: bead_id.to_string(),
        commit,
        obligations: obligations_status,
        remaining_assumptions: vec![
            "Fjall fsync correctness treated as external dependency".to_string(),
        ],
        verified: HashMap::new(),
    };

    let yaml = serde_saphyr::to_string(&evidence).map_err(|e| e.to_string())?;

    let path = output_dir.join("proof.yaml");
    std::fs::write(&path, &yaml).map_err(|e| e.to_string())?;

    Ok(path)
}
