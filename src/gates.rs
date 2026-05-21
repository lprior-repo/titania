//! Gate command wrappers for xtask command-center gates.

#![allow(dead_code)]

use crate::evidence::{GateEvidence, Result, run_gate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    Fmt,
    Check,
    Clippy,
    Nextest,
    ForbiddenScan,
    HotpathScan,
    Miri,
    Mutants,
    LlvmCov,
    FuzzBuild,
    SupplyChain,
    FuzzSmoke,
    Coverage,
    MutantsSmoke,
    BenchBuild,
    FeaturePowerset,
    SourceLength,
    Maxperf,
}

impl Gate {
    pub fn name(self) -> &'static str {
        match self {
            Gate::Fmt => "fmt",
            Gate::Check => "check",
            Gate::Clippy => "clippy",
            Gate::Nextest => "nextest",
            Gate::ForbiddenScan => "forbidden-scan",
            Gate::HotpathScan => "hotpath-scan",
            Gate::Miri => "miri",
            Gate::Mutants => "mutants",
            Gate::LlvmCov => "llvm-cov",
            Gate::FuzzBuild => "fuzz-build",
            Gate::SupplyChain => "supply-chain",
            Gate::FuzzSmoke => "fuzz-smoke",
            Gate::Coverage => "coverage",
            Gate::MutantsSmoke => "mutants-smoke",
            Gate::BenchBuild => "bench-build",
            Gate::FeaturePowerset => "feature-powerset",
            Gate::SourceLength => "source-length",
            Gate::Maxperf => "maxperf",
        }
    }

    pub fn command(self) -> Vec<String> {
        command_words(self)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    pub fn evidence_file(self) -> String {
        format!("{}.yaml", self.name())
    }
}

fn command_words(gate: Gate) -> &'static [&'static str] {
    match gate {
        Gate::Fmt => &["cargo", "+nightly", "fmt", "--all"],
        Gate::Check => &["moon", "run", ":check"],
        Gate::Clippy => &["cargo", "+nightly", "clippy", "--workspace"],
        Gate::Nextest => &["cargo", "nextest", "run", "--workspace"],
        Gate::ForbiddenScan => &["bash", "scripts/forbidden-scan.sh"],
        Gate::HotpathScan => &["bash", "scripts/hotpath-scan.sh"],
        Gate::Miri => &["cargo", "+nightly", "miri", "test", "--workspace"],
        Gate::Mutants => &["cargo", "mutants", "--package", "vb_cli"],
        Gate::LlvmCov => &["cargo", "llvm-cov"],
        Gate::FuzzBuild => &["cargo", "fuzz", "build"],
        Gate::SupplyChain => &["moon", "run", ":supply-chain"],
        Gate::FuzzSmoke => &["moon", "run", ":fuzz-smoke"],
        Gate::Coverage => &["moon", "run", ":coverage"],
        Gate::MutantsSmoke => &["moon", "run", ":mutants-smoke"],
        Gate::BenchBuild => &["moon", "run", ":bench-build"],
        Gate::FeaturePowerset => &["moon", "run", ":feature-powerset"],
        Gate::SourceLength => &["bash", "scripts/check-source-length.sh"],
        Gate::Maxperf => &["moon", "run", ":maxperf"],
    }
}

fn run_named_gate(gate: Gate, bead_id: Option<&str>) -> Result<GateEvidence> {
    let evidence_path = crate::evidence::evidence_path(bead_id.unwrap_or("default"), gate.name());
    run_gate(gate.name(), &gate.command(), &evidence_path)
}

pub fn run_fmt_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Fmt, bead_id)
}
pub fn run_check_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Check, bead_id)
}
pub fn run_clippy_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Clippy, bead_id)
}
pub fn run_nextest_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Nextest, bead_id)
}
pub fn run_forbidden_scan_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::ForbiddenScan, bead_id)
}
pub fn run_hotpath_scan_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::HotpathScan, bead_id)
}
pub fn run_miri_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Miri, bead_id)
}
pub fn run_mutants_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Mutants, bead_id)
}
pub fn run_llvm_cov_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::LlvmCov, bead_id)
}
pub fn run_fuzz_build_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::FuzzBuild, bead_id)
}
pub fn run_supply_chain_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::SupplyChain, bead_id)
}
pub fn run_fuzz_smoke_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::FuzzSmoke, bead_id)
}
pub fn run_coverage_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Coverage, bead_id)
}
pub fn run_mutants_smoke_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::MutantsSmoke, bead_id)
}
pub fn run_bench_build_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::BenchBuild, bead_id)
}
pub fn run_feature_powerset_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::FeaturePowerset, bead_id)
}
pub fn run_source_length_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::SourceLength, bead_id)
}
pub fn run_maxperf_gate(bead_id: Option<&str>) -> Result<GateEvidence> {
    run_named_gate(Gate::Maxperf, bead_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const ALL_GATES: [Gate; 18] = [
        Gate::Fmt,
        Gate::Check,
        Gate::Clippy,
        Gate::Nextest,
        Gate::ForbiddenScan,
        Gate::HotpathScan,
        Gate::Miri,
        Gate::Mutants,
        Gate::LlvmCov,
        Gate::FuzzBuild,
        Gate::SupplyChain,
        Gate::FuzzSmoke,
        Gate::Coverage,
        Gate::MutantsSmoke,
        Gate::BenchBuild,
        Gate::FeaturePowerset,
        Gate::SourceLength,
        Gate::Maxperf,
    ];

    #[test]
    fn gate_names_and_evidence_files_match_expected_identifiers() {
        let names: Vec<_> = ALL_GATES.iter().map(|gate| gate.name()).collect();
        assert_eq!(
            names,
            vec![
                "fmt",
                "check",
                "clippy",
                "nextest",
                "forbidden-scan",
                "hotpath-scan",
                "miri",
                "mutants",
                "llvm-cov",
                "fuzz-build",
                "supply-chain",
                "fuzz-smoke",
                "coverage",
                "mutants-smoke",
                "bench-build",
                "feature-powerset",
                "source-length",
                "maxperf",
            ]
        );
        for gate in ALL_GATES {
            assert_eq!(gate.evidence_file(), format!("{}.yaml", gate.name()));
        }
    }

    #[test]
    fn representative_gate_commands_match_contract() {
        assert_eq!(
            Gate::Fmt.command(),
            vec!["cargo", "+nightly", "fmt", "--all"]
        );
        assert_eq!(
            Gate::Clippy.command(),
            vec!["cargo", "+nightly", "clippy", "--workspace"]
        );
        assert_eq!(
            Gate::Miri.command(),
            vec!["cargo", "+nightly", "miri", "test", "--workspace"]
        );
        assert_eq!(Gate::Check.command(), vec!["moon", "run", ":check"]);
    }

    #[test]
    fn every_gate_has_a_command_and_yaml_evidence_file() {
        for gate in ALL_GATES {
            assert_ne!(gate.command(), Vec::<String>::new());
            assert_eq!(gate.evidence_file(), format!("{}.yaml", gate.name()));
        }
    }

    #[test]
    fn gate_runners_return_named_evidence_when_bead_is_provided() {
        type GateRunner = fn(Option<&str>) -> Result<GateEvidence>;
        let runners: [(GateRunner, &str); 18] = [
            (run_fmt_gate, "fmt"),
            (run_check_gate, "check"),
            (run_clippy_gate, "clippy"),
            (run_nextest_gate, "nextest"),
            (run_forbidden_scan_gate, "forbidden-scan"),
            (run_hotpath_scan_gate, "hotpath-scan"),
            (run_miri_gate, "miri"),
            (run_mutants_gate, "mutants"),
            (run_llvm_cov_gate, "llvm-cov"),
            (run_fuzz_build_gate, "fuzz-build"),
            (run_supply_chain_gate, "supply-chain"),
            (run_fuzz_smoke_gate, "fuzz-smoke"),
            (run_coverage_gate, "coverage"),
            (run_mutants_smoke_gate, "mutants-smoke"),
            (run_bench_build_gate, "bench-build"),
            (run_feature_powerset_gate, "feature-powerset"),
            (run_source_length_gate, "source-length"),
            (run_maxperf_gate, "maxperf"),
        ];
        for (runner, name) in runners {
            assert_eq!(
                runner(Some("vb-test")).map(|e| e.gate_name),
                Ok(name.to_string())
            );
        }
    }

    #[test]
    fn run_fmt_gate_uses_default_bead_when_bead_is_omitted() {
        assert_eq!(
            run_fmt_gate(None).map(|evidence| evidence.log),
            Ok(PathBuf::from(".evidence/default/fmt.log"))
        );
    }
}
