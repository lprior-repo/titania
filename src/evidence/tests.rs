#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn gate_profile_evidence_files_and_gates_are_stable() {
        assert_eq!(GateProfile::AiFast.evidence_file(), "ai-fast.yaml");
        assert_eq!(GateProfile::AiDeep.evidence_file(), "ai-deep.yaml");
        assert_eq!(GateProfile::AiRelease.evidence_file(), "ai-release.yaml");
        assert_eq!(GateProfile::AiFast.gates(), &["fmt", "check", "clippy", "nextest", "forbidden-scan", "hotpath-scan"]);
        assert_eq!(GateProfile::AiDeep.gates(), &["miri", "mutants", "llvm-cov", "fuzz-build"]);
        assert!(GateProfile::AiRelease.gates().contains(&"maxperf"));
    }

    #[test]
    fn evidence_path_stays_under_bead_directory() {
        assert_eq!(evidence_path("vb-kkvb", "fmt"), PathBuf::from(".evidence/vb-kkvb/fmt.yaml"));
    }

    #[test]
    fn failed_gate_explains_failure_with_hint_and_repair() {
        let evidence = GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo fmt".to_string(),
            exit_code: 1,
            log: PathBuf::from("fmt.log"),
            status: GateStatus::Fail,
            why_failed: None,
        };
        let why = explain_failure(&evidence);
        assert!(why.is_some(), "failed evidence explains failure");
        if let Some(why) = why {
            assert_eq!(why.gate_name, "fmt");
            assert!(!why.hint.is_empty());
            assert!(!why.repair_command.is_empty());
        }
    }

    #[test]
    fn release_bead_id_accepts_only_supported_release_bead() {
        assert_eq!(ReleaseBeadId::parse("vb-nf2u"), Ok(ReleaseBeadId::VbNf2u));
        assert!(ReleaseBeadId::parse("vb-other").is_err());
    }
}
