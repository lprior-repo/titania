//! Lane failure classification and process termination reasons.
//!
//! When a lane does not produce a clean or finding outcome, the failure is
//! categorized so that the aggregator can decide how to report it.

use serde::{Deserialize, Serialize};

use crate::error::FailureError;

/// Why a process terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessTermination {
    /// Process exited with a specific exit code.
    Exited { code: i32 },
    /// Process was terminated by a signal (Unix only).
    ///
    /// Signal numbers are in the range 1–31 (SIGHUP through SIGSYS).
    /// Windows processes killed via `TerminateProcess` appear as
    /// `Exited { code: 1 }` — there is no signal concept on Windows.
    Signaled { signal: i32 },
    /// Process exceeded its configured timeout.
    TimedOut,
    /// Process exceeded its memory limit.
    MemoryLimitExceeded,
    /// The process could not be spawned (e.g. binary not found).
    SpawnFailed,
}

impl ProcessTermination {
    /// Construct a signal-based termination.
    ///
    /// # Errors
    /// - [`FailureError::InvalidSignal`] if the signal is outside 1–31.
    pub fn signaled(signal: i32) -> Result<Self, FailureError> {
        if !(1..=31).contains(&signal) {
            return Err(FailureError::InvalidSignal(signal));
        }
        Ok(ProcessTermination::Signaled { signal })
    }

    /// Whether this represents a normal (non-error) exit.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, ProcessTermination::Exited { code } if *code == 0)
    }

    /// If this is an `Exited` variant, return the exit code.
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            ProcessTermination::Exited { code } => Some(*code),
            _ => None,
        }
    }
}

/// Classification of a lane failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
#[serde(rename_all = "snake_case")]
pub enum LaneFailure {
    /// Infrastructure problem — tool binary missing, ABI mismatch, etc.
    InfraFailure { tool: String, reason: String },
    /// Tool ran but terminated abnormally.
    ToolFailure { tool: String, termination: ProcessTermination },
    /// Resource constraint hit — memory limit, file descriptor limit, etc.
    ResourceFailure { tool: String, limit: String },
    /// Tool failed but the cause is suspicious (e.g. intermittent,
    /// non-reproducible). The `evidence` field contains additional context.
    SuspiciousFailure { tool: String, evidence: String },
}

impl LaneFailure {
    /// The tool that failed, if known.
    #[must_use]
    pub fn tool(&self) -> Option<&str> {
        match self {
            LaneFailure::InfraFailure { tool, .. }
            | LaneFailure::ToolFailure { tool, .. }
            | LaneFailure::ResourceFailure { tool, .. }
            | LaneFailure::SuspiciousFailure { tool, .. } => Some(tool),
        }
    }

    /// Whether this is an infrastructure issue (not a code problem).
    #[must_use]
    pub fn is_infra(&self) -> bool {
        matches!(self, LaneFailure::InfraFailure { .. })
    }

    /// Whether this is a resource constraint.
    #[must_use]
    pub fn is_resource(&self) -> bool {
        matches!(self, LaneFailure::ResourceFailure { .. })
    }
}
