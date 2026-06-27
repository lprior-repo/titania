//! Lane types: which quality check ran, its outcome, and failure categories.

use serde::{Deserialize, Serialize};

/// Which gate scope to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateScope {
    /// Fast AI repair loop: fmt, check, clippy(source), semgrep, panic-scan.
    Edit,
    /// Before push: edit + tests + supply chain + feature matrix.
    Prepush,
    /// CI gate: prepush + cargo mutants.
    Full,
    /// Release check: full + cargo build --release.
    Release,
}

impl GateScope {
    /// Returns the lanes that run in this scope, in execution order.
    #[must_use]
    pub const fn lanes(self) -> &'static [Lane] {
        match self {
            Self::Edit => &[
                Lane::Fmt,
                Lane::Check,
                Lane::Clippy,
                Lane::Semgrep,
                Lane::PanicAssertScan,
            ],
            Self::Prepush => &[
                Lane::Fmt,
                Lane::Check,
                Lane::Clippy,
                Lane::Semgrep,
                Lane::PanicAssertScan,
                Lane::Test,
                Lane::SupplyChain,
                Lane::FeatureMatrix,
            ],
            Self::Full => &[
                Lane::Fmt,
                Lane::Check,
                Lane::Clippy,
                Lane::Semgrep,
                Lane::PanicAssertScan,
                Lane::Test,
                Lane::SupplyChain,
                Lane::FeatureMatrix,
                Lane::Mutants,
            ],
            Self::Release => &[
                Lane::Fmt,
                Lane::Check,
                Lane::Clippy,
                Lane::Semgrep,
                Lane::PanicAssertScan,
                Lane::Test,
                Lane::SupplyChain,
                Lane::FeatureMatrix,
                Lane::Mutants,
                Lane::ArtifactBuild,
            ],
        }
    }
}

/// A named quality enforcement lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lane {
    /// Layer 0: `cargo fmt --check`
    Fmt,
    /// Layer 1: `cargo check --frozen` + rustc lints
    Check,
    /// Layer 2: `cargo clippy --lib --bins --frozen` (source-only)
    Clippy,
    /// Layer 3: semgrep structural source rules
    Semgrep,
    /// Layer 4: production panic/assert + build-script scan
    PanicAssertScan,
    /// Layer 5: `cargo test --frozen`
    Test,
    /// Layer 6: cargo audit/deny/vet/geiger/machete
    SupplyChain,
    /// Layer 7: `cargo hack --feature-powerset`
    FeatureMatrix,
    /// Layer 8: `cargo mutants`
    Mutants,
    /// Layer 9: `cargo build --release`
    ArtifactBuild,
}

impl Lane {
    /// Whether this lane requires successful compilation (Layer 1) to run.
    #[must_use]
    pub const fn depends_on_compilation(self) -> bool {
        matches!(
            self,
            Self::Clippy | Self::Test | Self::FeatureMatrix | Self::Mutants | Self::ArtifactBuild
        )
    }
}

/// The outcome of a single lane.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LaneOutcome {
    /// Lane passed cleanly.
    Clean { evidence: LaneEvidence },
    /// Lane found policy violations.
    Findings(Box<[crate::finding::Finding]>),
    /// Lane itself failed (tool crashed, missing, timeout).
    Failed(LaneFailure),
    /// Lane was not run.
    Skipped(SkipReason),
}

/// Why a lane was skipped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SkipReason {
    /// Layer 1 (check) failed; compilation-dependent lanes skip.
    PriorCompilationFailure,
    /// The selected scope did not include this lane.
    NotSelectedByScope,
    /// Not applicable (e.g. no unsafe → certain scans N/A).
    NotApplicable,
    /// Policy explicitly disabled this lane.
    PolicyDisabled,
}

/// Reproducible evidence of what a lane ran and found.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneEvidence {
    /// The command that was executed.
    pub command: CommandEvidence,
    /// Tool version string.
    pub tool_version: String,
    /// How the process terminated.
    pub exit_status: ProcessTermination,
    /// Digest of the parsed result (findings or pass).
    pub parsed_result_digest: crate::Digest,
}

/// The exact command evidence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandEvidence {
    /// Resolved tool name or path.
    pub executable: String,
    /// Full argv.
    pub argv: Box<[String]>,
}

/// How a lane's process terminated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProcessTermination {
    /// Normal exit with code.
    Exited { code: i32 },
    /// Killed by signal.
    Signaled { signal: i32 },
    /// Exceeded time budget.
    TimedOut,
    /// Exceeded memory budget.
    MemoryLimitExceeded,
    /// Failed to spawn.
    SpawnFailed,
}

/// Why a lane itself failed (not a code finding).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LaneFailure {
    /// Infrastructure: missing binary, version mismatch.
    InfraFailure { tool: String, reason: String },
    /// Tool crashed on input.
    ToolFailure {
        tool: String,
        termination: ProcessTermination,
    },
    /// Resource limit hit: timeout, memory cap.
    ResourceFailure { tool: String, limit: String },
    /// Likely hostile/pathological input.
    SuspiciousFailure { tool: String, evidence: String },
}

/// A per-lane entry in the `QualityReceipt`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneReceipt {
    /// Which lane.
    pub lane: Lane,
    /// Digest of the lane evidence.
    pub evidence_digest: crate::Digest,
    /// Whether the lane was clean.
    pub clean: bool,
}
