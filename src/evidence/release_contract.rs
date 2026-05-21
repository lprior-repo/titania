//
// This module provides the evidence bundle types (GateEvidence, WhyFailed, GateStatus)
// and orchestration functions (run_gate, run_profile, explain_failure, validate_evidence_dir).
//
// # Error Taxonomy
//
// All fallible operations return `Result<T, Error>` with explicit error variants:
//
// - `Error::GateTimeout` — gate exceeded its time bound
// - `Error::GateFailed` — underlying command returned non-zero
// - `Error::MissingEvidence` — evidence file absent (fail-closed trigger)
// - `Error::EvidenceWriteFailed` — YAML serialization or file write error
// - `Error::SubcommandNotFound` — requested xtask subcommand does not exist
// - `Error::BeadDirectoryCreationFailed` — could not create `.evidence/<bead>/` directory
// - `Error::YamlSerializationFailed` — saphyr error during evidence serialization
// - `Error::UpstreamMoonFailed` — moon run task returned non-zero
// - `Error::UpstreamJustFailed` — just recipe returned non-zero

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use vb_ui_snapshot::layout_kernel::{
    Rect, SelectedIndicator, chip_is_readable, is_clipped, is_out_of_bounds, overlap_area_px,
    selected_state_is_visible,
};

const VB_NF2U: &str = "vb-nf2u";
const CANONICAL_SCREENS: [&str; 8] = [
    "execution_overview",
    "workflow_graph_authoring",
    "execution_details",
    "verification_certificate",
    "replay_theater",
    "incident_failure",
    "action_registry",
    "storage_doctor_ai_context",
];
const REQUIRED_UI_SUBGATES: [&str; 6] = [
    "ui_snapshot",
    "layout_readability",
    "redaction",
    "negative_fixture",
    "deterministic_capture",
    "evidence_shape",
];
const REQUIRED_LAYOUT_CHECKS: [&str; 7] = [
    "Overlap",
    "Clipping",
    "Bounds",
    "ChipReadability",
    "SelectedState",
    "FixtureArtifactProvenance",
    "Redaction",
];
const AI_FAST_GATES: &[&str] = &[
    "fmt",
    "check",
    "clippy",
    "nextest",
    "forbidden-scan",
    "hotpath-scan",
];
const AI_RELEASE_GATES: &[&str] = &[
    "check",
    "test",
    "supply-chain",
    "miri",
    "fuzz-smoke",
    "coverage",
    "mutants-smoke",
    "bench-build",
    "feature-powerset",
    "source-length",
    "maxperf",
];
const REDACTION_CLASSES: [(&str, &str); 6] = [
    ("sentinel", "[REDACTED:sentinel]"),
    ("api_key", "[REDACTED:api_key]"),
    ("token", "[REDACTED:token]"),
    ("password", "[REDACTED:password]"),
    ("idempotency_key", "[REDACTED:idempotency_key]"),
    ("tainted_fixture_value", "[REDACTED:tainted_fixture_value]"),
];
const NEGATIVE_FIXTURE_ROOT: &str = "target/vb-nf2u-negative-fixtures";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseBeadId {
    VbNf2u,
}

impl ReleaseBeadId {
    pub fn parse(value: &str) -> std::result::Result<Self, UiReleaseGateError> {
        match value {
            VB_NF2U => Ok(Self::VbNf2u),
            _ => Err(UiReleaseGateError::ReleaseProfileIncomplete {
                code: "release_profile_incomplete",
                bead_id: "unknown",
                missing_subgates: REQUIRED_UI_SUBGATES.to_vec(),
                action: "reject unknown bead id before generating release evidence",
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::VbNf2u => VB_NF2U,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NegativeFixtureWorkflow {
    Required,
    Missing,
    Observed {
        fixture_id: &'static str,
        expected_gate: &'static str,
        actual_status: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReleaseArtifactWorkflow {
    None,
    Text {
        path: &'static str,
        text: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReleaseParityClaim {
    FixtureBacked,
    LiveCoreRuntime(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FixtureReadState {
    Present(String),
    Missing(PathBuf),
}

/// UI release-gate failures required by the vb-nf2u contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiReleaseGateError {
    InvalidScreenInventory {
        code: &'static str,
        screen_id_or_count: &'static str,
        reason: &'static str,
        action: &'static str,
    },
    UnreachableScreen {
        code: &'static str,
        screen_id: &'static str,
        mapping_edge: &'static str,
        action: &'static str,
    },
    SnapshotDeterminismViolation {
        code: &'static str,
        screen_id: &'static str,
        expected_field: &'static str,
        expected_value: &'static str,
        actual_field: &'static str,
        actual_value: &'static str,
        action: &'static str,
    },
    MissingEvidence {
        code: &'static str,
        screen_id: &'static str,
        artifact_path: &'static str,
        evidence_kind: &'static str,
        action: &'static str,
    },
    LayoutViolation {
        code: &'static str,
        screen_id: &'static str,
        control_id: &'static str,
        predicate: &'static str,
        bounds: &'static str,
        action: &'static str,
    },
    RedactionViolation {
        code: &'static str,
        screen_id: &'static str,
        artifact_path: &'static str,
        secret_class: &'static str,
        redacted_sample: &'static str,
        action: &'static str,
    },
    FalsePassFixtureViolation {
        code: &'static str,
        fixture_id: &'static str,
        expected_gate: &'static str,
        actual_status: &'static str,
        action: &'static str,
    },
    ReleaseProfileIncomplete {
        code: &'static str,
        bead_id: &'static str,
        missing_subgates: Vec<&'static str>,
        action: &'static str,
    },
    CoreParityUnsupported {
        code: &'static str,
        claim: &'static str,
        blocker: &'static str,
        action: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiReleaseToolingLaneKind {
    ExecutableGate,
    ExternalMachineGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiReleaseToolingLane {
    pub name: &'static str,
    pub command: &'static str,
    pub kind: UiReleaseToolingLaneKind,
    pub blocker: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiReleaseInventory {
    screen_ids: Vec<&'static str>,
    missing_fixture_edge: Option<&'static str>,
}

impl UiReleaseInventory {
    pub fn from_screen_ids<const N: usize>(screen_ids: [&'static str; N]) -> Self {
        Self {
            screen_ids: screen_ids.into_iter().collect(),
            missing_fixture_edge: None,
        }
    }

    pub fn without_fixture_edge(mut self, screen_id: &'static str) -> Self {
        self.missing_fixture_edge = Some(screen_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDeterminismConfig {
    screen_id: &'static str,
    source: SnapshotTimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SnapshotTimeSource {
    Fixed,
    WallClock,
}

impl SnapshotDeterminismConfig {
    pub fn wall_clock_for_screen(screen_id: &'static str) -> Self {
        Self {
            screen_id,
            source: SnapshotTimeSource::WallClock,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotGuard {
    marker: &'static str,
}

impl ReleaseSnapshotGuard {
    pub fn evidence_marker(&self) -> &'static str {
        self.marker
    }
}
