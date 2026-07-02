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
    /// Construct an exit-code termination.
    #[must_use]
    pub const fn exited(code: i32) -> Self {
        Self::Exited { code }
    }

    /// Construct a timeout termination.
    #[must_use]
    pub const fn timed_out() -> Self {
        Self::TimedOut
    }

    /// Construct a memory-limit termination.
    #[must_use]
    pub const fn memory_limit_exceeded() -> Self {
        Self::MemoryLimitExceeded
    }

    /// Construct a spawn-failed termination.
    #[must_use]
    pub const fn spawn_failed() -> Self {
        Self::SpawnFailed
    }

    /// Construct a signal-based termination.
    ///
    /// # Errors
    /// - [`FailureError::InvalidSignal`] if the signal is outside 1–31.
    pub fn signaled(signal: i32) -> Result<Self, FailureError> {
        if !(1_i32..=31_i32).contains(&signal) {
            return Err(FailureError::InvalidSignal(signal));
        }
        Ok(Self::Signaled { signal })
    }

    /// Whether this represents a normal (non-error) exit.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Exited { code } if code == 0)
    }

    /// If this is an `Exited` variant, return the exit code.
    #[must_use]
    pub const fn exit_code(self) -> Option<i32> {
        match self {
            Self::Exited { code } => Some(code),
            _ => None,
        }
    }
    /// If this is a `Signaled` variant, return the signal number.
    #[must_use]
    pub const fn signal(self) -> Option<i32> {
        match self {
            Self::Signaled { signal } => Some(signal),
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
    /// Construct an infrastructure failure.
    #[must_use]
    pub const fn infra_failure(tool: String, reason: String) -> Self {
        Self::InfraFailure { tool, reason }
    }

    /// Construct a tool termination failure.
    #[must_use]
    pub const fn tool_failure(tool: String, termination: ProcessTermination) -> Self {
        Self::ToolFailure { tool, termination }
    }

    /// Construct a resource failure.
    #[must_use]
    pub const fn resource_failure(tool: String, limit: String) -> Self {
        Self::ResourceFailure { tool, limit }
    }

    /// Construct a suspicious failure.
    #[must_use]
    pub const fn suspicious_failure(tool: String, evidence: String) -> Self {
        Self::SuspiciousFailure { tool, evidence }
    }

    /// The tool that failed, if known.
    #[must_use]
    pub fn tool(&self) -> &str {
        match self {
            Self::InfraFailure { tool, .. }
            | Self::ToolFailure { tool, .. }
            | Self::ResourceFailure { tool, .. }
            | Self::SuspiciousFailure { tool, .. } => tool,
        }
    }

    /// Whether this is an infrastructure issue (not a code problem).
    #[must_use]
    pub const fn is_infra(&self) -> bool {
        matches!(self, Self::InfraFailure { .. })
    }

    /// Whether this is a resource constraint.
    #[must_use]
    pub const fn is_resource(&self) -> bool {
        matches!(self, Self::ResourceFailure { .. })
    }
}
