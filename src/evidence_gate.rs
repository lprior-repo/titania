//! Evidence gate types and validation for supply-chain, API, semver, bloat, and performance evidence.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Required metadata fields for benchmark evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkEvidence {
    pub name: String,
    pub baseline: Option<String>,
    pub result: Option<String>,
    pub environment: Option<String>,
    pub command: Option<String>,
}

impl BenchmarkEvidence {
    pub fn has_baseline(&self) -> bool {
        self.baseline.as_ref().is_some_and(|b| !b.is_empty())
    }

    pub fn has_result(&self) -> bool {
        self.result.as_ref().is_some_and(|r| !r.is_empty())
    }

    pub fn has_environment(&self) -> bool {
        self.environment.as_ref().is_some_and(|e| !e.is_empty())
    }

    pub fn has_command(&self) -> bool {
        self.command.as_ref().is_some_and(|c| !c.is_empty())
    }

    pub fn is_complete(&self) -> bool {
        self.has_baseline() && self.has_result() && self.has_environment() && self.has_command()
    }
}

/// Supply-chain audit result from a single tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditResult {
    pub tool: String,
    pub exit_code: Option<i32>,
    pub output_path: Option<PathBuf>,
    pub passed: bool,
    pub notes: String,
}

/// Evidence bundle collecting all release evidence categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub supply_chain_audits: Vec<AuditResult>,
    pub api_surface: Option<ApiSurfaceRecord>,
    pub semver_record: Option<SemverRecord>,
    pub bloat_analysis: Option<BloatRecord>,
    pub benchmark_evidence: Vec<BenchmarkEvidence>,
    pub kernel_paths_covered: Vec<String>,
    pub captured_at: String,
    pub toolchain: String,
    pub host_cpu: String,
}

impl EvidenceBundle {
    pub fn new(toolchain: String, host_cpu: String, captured_at: String) -> Self {
        Self {
            supply_chain_audits: Vec::new(),
            api_surface: None,
            semver_record: None,
            bloat_analysis: None,
            benchmark_evidence: Vec::new(),
            kernel_paths_covered: Vec::new(),
            captured_at,
            toolchain,
            host_cpu,
        }
    }

    /// Check if all required evidence categories are present.
    pub fn is_complete(&self) -> bool {
        !self.supply_chain_audits.is_empty()
            && self.api_surface.is_some()
            && self.semver_record.is_some()
            && self.bloat_analysis.is_some()
            && !self.benchmark_evidence.is_empty()
            && !self.kernel_paths_covered.is_empty()
    }

    /// Check if any supply-chain audit failed.
    pub fn has_audit_failure(&self) -> bool {
        self.supply_chain_audits.iter().any(|a| !a.passed)
    }

    /// Check if any benchmark evidence is missing baseline metadata.
    pub fn has_missing_benchmark_baseline(&self) -> bool {
        self.benchmark_evidence.iter().any(|b| !b.has_baseline())
    }

    /// Check if any benchmark evidence is missing required metadata.
    pub fn has_incomplete_benchmark_metadata(&self) -> bool {
        self.benchmark_evidence.iter().any(|b| !b.is_complete())
    }

    /// Validate all evidence gates. Returns a list of gate failures.
    pub fn validate_gates(&self) -> Vec<EvidenceGateFailure> {
        let mut failures = Vec::new();

        if self.supply_chain_audits.is_empty() {
            failures.push(EvidenceGateFailure::MissingSupplyChainEvidence);
        }

        if self.has_audit_failure() {
            let failed_tools: Vec<&str> = self
                .supply_chain_audits
                .iter()
                .filter(|a| !a.passed)
                .map(|a| a.tool.as_str())
                .collect();
            failures.push(EvidenceGateFailure::AuditFailure {
                tools: failed_tools.iter().map(|s| s.to_string()).collect(),
            });
        }

        if self.api_surface.is_none() {
            failures.push(EvidenceGateFailure::MissingApiSurfaceEvidence);
        }

        if self.semver_record.is_none() {
            failures.push(EvidenceGateFailure::MissingSemverEvidence);
        }

        if self.bloat_analysis.is_none() {
            failures.push(EvidenceGateFailure::MissingBloatAnalysis);
        }

        if self.benchmark_evidence.is_empty() {
            failures.push(EvidenceGateFailure::MissingBenchmarkEvidence);
        }

        if self.has_missing_benchmark_baseline() {
            let missing: Vec<&str> = self
                .benchmark_evidence
                .iter()
                .filter(|b| !b.has_baseline())
                .map(|b| b.name.as_str())
                .collect();
            failures.push(EvidenceGateFailure::MissingBenchmarkBaseline {
                benchmarks: missing.iter().map(|s| s.to_string()).collect(),
            });
        }

        if self.has_incomplete_benchmark_metadata() {
            let incomplete: Vec<&str> = self
                .benchmark_evidence
                .iter()
                .filter(|b| !b.is_complete())
                .map(|b| b.name.as_str())
                .collect();
            failures.push(EvidenceGateFailure::IncompleteBenchmarkMetadata {
                benchmarks: incomplete.iter().map(|s| s.to_string()).collect(),
            });
        }

        let required_kernel_paths = [
            "yaml_parse",
            "compile_validate",
            "expression",
            "runtime_core",
            "storage_ipc",
            "generated_mode",
        ];
        let missing_kernel: Vec<&str> = required_kernel_paths
            .iter()
            .filter(|p| !self.kernel_paths_covered.iter().any(|k| k.contains(**p)))
            .copied()
            .collect();
        if !missing_kernel.is_empty() {
            failures.push(EvidenceGateFailure::MissingKernelPathEvidence {
                paths: missing_kernel.iter().map(|s| s.to_string()).collect(),
            });
        }

        failures
    }

    /// Returns true if all evidence gates pass.
    pub fn all_gates_pass(&self) -> bool {
        self.validate_gates().is_empty()
    }
}

/// API surface record for a single crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSurfaceRecord {
    pub crate_name: String,
    pub version: String,
    pub public_item_count: usize,
}

/// Semver stability record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemverRecord {
    pub crate_name: String,
    pub current_version: String,
    pub previous_version: Option<String>,
    pub breaking_changes: Vec<String>,
}

/// Binary bloat analysis record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloatRecord {
    pub binary_path: String,
    pub total_size_bytes: u64,
    pub top_contributors: Vec<BloatContributor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloatContributor {
    pub name: String,
    pub size_bytes: u64,
    pub percentage: f64,
}

/// Evidence gate failure types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceGateFailure {
    MissingSupplyChainEvidence,
    AuditFailure { tools: Vec<String> },
    MissingApiSurfaceEvidence,
    MissingSemverEvidence,
    MissingBloatAnalysis,
    MissingBenchmarkEvidence,
    MissingBenchmarkBaseline { benchmarks: Vec<String> },
    IncompleteBenchmarkMetadata { benchmarks: Vec<String> },
    MissingKernelPathEvidence { paths: Vec<String> },
}

impl std::fmt::Display for EvidenceGateFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSupplyChainEvidence => {
                write!(f, "supply-chain evidence is missing")
            }
            Self::AuditFailure { tools } => {
                write!(f, "audit failure in tools: {}", tools.join(", "))
            }
            Self::MissingApiSurfaceEvidence => {
                write!(f, "public API surface evidence is missing")
            }
            Self::MissingSemverEvidence => {
                write!(f, "semver stability evidence is missing")
            }
            Self::MissingBloatAnalysis => {
                write!(f, "binary bloat analysis is missing")
            }
            Self::MissingBenchmarkEvidence => {
                write!(f, "benchmark evidence is missing")
            }
            Self::MissingBenchmarkBaseline { benchmarks } => {
                write!(
                    f,
                    "benchmark baseline missing for: {}",
                    benchmarks.join(", ")
                )
            }
            Self::IncompleteBenchmarkMetadata { benchmarks } => {
                write!(
                    f,
                    "incomplete benchmark metadata for: {}",
                    benchmarks.join(", ")
                )
            }
            Self::MissingKernelPathEvidence { paths } => {
                write!(f, "kernel path evidence missing for: {}", paths.join(", "))
            }
        }
    }
}

/// Known kernel benchmark groups that must have evidence.
pub fn required_kernel_groups() -> &'static [&'static str] {
    &[
        "yaml_parse",
        "compile_validate",
        "expression",
        "runtime_core",
        "storage_ipc",
        "generated_mode",
    ]
}

/// Parse criterion benchmark output into BenchmarkEvidence records.
pub fn parse_criterion_output(output: &str) -> Vec<BenchmarkEvidence> {
    let mut records = Vec::new();
    for line in output.lines() {
        if line.contains("time:") && line.contains("BenchmarkId") {
            let name = extract_benchmark_name(line).unwrap_or_default();
            records.push(BenchmarkEvidence {
                name,
                baseline: Some("vb-current".to_string()),
                result: extract_time_value(line),
                environment: None,
                command: None,
            });
        }
    }
    records
}

fn extract_benchmark_name(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("BenchmarkId(")?;
    let (name, _) = rest.split_once(')')?;
    Some(name.to_string())
}

fn extract_time_value(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("time: [")?;
    let (time, _) = rest.split_once(']')?;
    Some(time.to_string())
}

/// Enrich benchmark evidence with environment and command metadata.
pub fn enrich_benchmark_evidence(
    evidence: &mut BenchmarkEvidence,
    command: &str,
    toolchain: &str,
    host_cpu: &str,
) {
    evidence.command = Some(command.to_string());
    evidence.environment = Some(format!("toolchain={toolchain};host_cpu={host_cpu}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_evidence_complete_has_all_fields() {
        let evidence = BenchmarkEvidence {
            name: "test".to_string(),
            baseline: Some("vb-current".to_string()),
            result: Some("100 ns".to_string()),
            environment: Some("toolchain=nightly;cpu=x86_64".to_string()),
            command: Some("cargo bench".to_string()),
        };
        assert!(evidence.is_complete());
    }

    #[test]
    fn benchmark_evidence_missing_baseline_is_incomplete() {
        let evidence = BenchmarkEvidence {
            name: "test".to_string(),
            baseline: None,
            result: Some("100 ns".to_string()),
            environment: Some("env".to_string()),
            command: Some("cmd".to_string()),
        };
        assert!(!evidence.is_complete());
        assert!(!evidence.has_baseline());
    }

    #[test]
    fn benchmark_evidence_empty_baseline_is_incomplete() {
        let evidence = BenchmarkEvidence {
            name: "test".to_string(),
            baseline: Some(String::new()),
            result: Some("100 ns".to_string()),
            environment: Some("env".to_string()),
            command: Some("cmd".to_string()),
        };
        assert!(!evidence.is_complete());
        assert!(!evidence.has_baseline());
    }

    #[test]
    fn evidence_bundle_validates_missing_supply_chain() {
        let bundle = EvidenceBundle::new(
            "nightly-2026-04-28".to_string(),
            "x86_64".to_string(),
            "2026-05-17".to_string(),
        );
        let failures = bundle.validate_gates();
        assert!(
            failures
                .iter()
                .any(|f| matches!(f, EvidenceGateFailure::MissingSupplyChainEvidence))
        );
    }

    #[test]
    fn evidence_bundle_validates_audit_failure_blocks_gate() {
        let mut bundle = EvidenceBundle::new(
            "nightly-2026-04-28".to_string(),
            "x86_64".to_string(),
            "2026-05-17".to_string(),
        );
        bundle.supply_chain_audits.push(AuditResult {
            tool: "cargo-deny".to_string(),
            exit_code: Some(1),
            output_path: None,
            passed: false,
            notes: "license failure".to_string(),
        });
        let failures = bundle.validate_gates();
        assert!(
            failures
                .iter()
                .any(|f| matches!(f, EvidenceGateFailure::AuditFailure { .. }))
        );
    }

    #[test]
    fn evidence_bundle_validates_missing_benchmark_baseline_blocks_speed_claim() {
        let mut bundle = EvidenceBundle::new(
            "nightly-2026-04-28".to_string(),
            "x86_64".to_string(),
            "2026-05-17".to_string(),
        );
        bundle.benchmark_evidence.push(BenchmarkEvidence {
            name: "fast_bench".to_string(),
            baseline: None,
            result: Some("10 ns".to_string()),
            environment: Some("env".to_string()),
            command: Some("cargo bench".to_string()),
        });
        let failures = bundle.validate_gates();
        assert!(
            failures
                .iter()
                .any(|f| matches!(f, EvidenceGateFailure::MissingBenchmarkBaseline { .. }))
        );
    }

    #[test]
    fn evidence_bundle_passes_with_complete_evidence() {
        let mut bundle = EvidenceBundle::new(
            "nightly-2026-04-28".to_string(),
            "x86_64".to_string(),
            "2026-05-17".to_string(),
        );
        bundle.supply_chain_audits.push(AuditResult {
            tool: "cargo-audit".to_string(),
            exit_code: Some(0),
            output_path: None,
            passed: true,
            notes: String::new(),
        });
        bundle.api_surface = Some(ApiSurfaceRecord {
            crate_name: "vb_core".to_string(),
            version: "0.1.0".to_string(),
            public_item_count: 42,
        });
        bundle.semver_record = Some(SemverRecord {
            crate_name: "vb_core".to_string(),
            current_version: "0.1.0".to_string(),
            previous_version: None,
            breaking_changes: Vec::new(),
        });
        bundle.bloat_analysis = Some(BloatRecord {
            binary_path: "target/release/velvet-ballastics".to_string(),
            total_size_bytes: 5_000_000,
            top_contributors: Vec::new(),
        });
        bundle.benchmark_evidence.push(BenchmarkEvidence {
            name: "yaml_parse".to_string(),
            baseline: Some("vb-current".to_string()),
            result: Some("100 ns".to_string()),
            environment: Some("env".to_string()),
            command: Some("cargo bench".to_string()),
        });
        bundle.kernel_paths_covered = required_kernel_groups()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(bundle.all_gates_pass());
    }

    #[test]
    fn parse_criterion_output_extracts_benchmark_names() {
        let output = "yaml_parse/parse_yaml_small;BenchmarkId(parse_yaml_small); time: [100.0 ns]";
        let records = parse_criterion_output(output);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "parse_yaml_small");
    }

    #[test]
    fn enrich_benchmark_evidence_adds_command_and_environment() {
        let mut evidence = BenchmarkEvidence {
            name: "test".to_string(),
            baseline: Some("vb-current".to_string()),
            result: Some("100 ns".to_string()),
            environment: None,
            command: None,
        };
        enrich_benchmark_evidence(
            &mut evidence,
            "cargo bench -- --save-baseline vb-current",
            "nightly-2026-04-28",
            "x86_64-unknown-linux-gnu",
        );
        assert!(evidence.command.is_some());
        assert!(evidence.environment.is_some());
        assert!(
            evidence
                .command
                .as_ref()
                .is_some_and(|c| c.contains("cargo bench"))
        );
        assert!(
            evidence
                .environment
                .as_ref()
                .is_some_and(|e| e.contains("toolchain=nightly-2026-04-28"))
        );
    }

    #[test]
    fn required_kernel_groups_lists_all_kernel_paths() {
        let groups = required_kernel_groups();
        assert!(groups.contains(&"yaml_parse"));
        assert!(groups.contains(&"compile_validate"));
        assert!(groups.contains(&"expression"));
        assert!(groups.contains(&"runtime_core"));
        assert!(groups.contains(&"storage_ipc"));
        assert!(groups.contains(&"generated_mode"));
        assert_eq!(groups.len(), 6);
    }

    #[test]
    fn evidence_gate_failure_display_formats() {
        let failure = EvidenceGateFailure::MissingBenchmarkBaseline {
            benchmarks: vec!["fast_bench".to_string()],
        };
        let display = format!("{failure}");
        assert!(display.contains("fast_bench"));
        assert!(display.contains("baseline"));
    }

    #[test]
    fn audit_failure_with_multiple_tools_lists_all() {
        let failure = EvidenceGateFailure::AuditFailure {
            tools: vec!["cargo-deny".to_string(), "cargo-vet".to_string()],
        };
        let display = format!("{failure}");
        assert!(display.contains("cargo-deny"));
        assert!(display.contains("cargo-vet"));
    }
}
