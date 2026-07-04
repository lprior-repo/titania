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
    Exited {
        /// Process exit code reported by the operating system.
        code: i32,
    },
    /// Process was terminated by a signal (Unix only).
    ///
    /// Signal numbers are in the range 1–31 (SIGHUP through SIGSYS).
    /// Windows processes killed via `TerminateProcess` appear as
    /// `Exited { code: 1 }` — there is no signal concept on Windows.
    Signaled {
        /// Unix signal number that terminated the process.
        signal: i32,
    },
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
        (1..=31)
            .contains(&signal)
            .then_some(Self::Signaled { signal })
            .ok_or(FailureError::InvalidSignal(signal))
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
}

/// Classification of a lane failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneFailure {
    /// Infrastructure problem — tool binary missing, ABI mismatch, etc.
    #[serde(rename = "infra_failure")]
    Infra {
        /// Tool that could not be invoked or trusted.
        tool: String,
        /// Stable reason explaining the infrastructure failure.
        reason: String,
    },
    /// Tool ran but terminated abnormally.
    #[serde(rename = "tool_failure")]
    Tool {
        /// Tool that ran and returned an abnormal termination.
        tool: String,
        /// Termination status observed from the tool process.
        termination: ProcessTermination,
    },
    /// Resource constraint hit — memory limit, file descriptor limit, etc.
    #[serde(rename = "resource_failure")]
    Resource {
        /// Tool that exceeded a resource limit.
        tool: String,
        /// Resource limit that was exceeded.
        limit: String,
    },
    /// Tool failed but the cause is suspicious (e.g. intermittent,
    /// non-reproducible). The `evidence` field contains additional context.
    #[serde(rename = "suspicious_failure")]
    Suspicious {
        /// Tool that produced suspicious failure evidence.
        tool: String,
        /// Additional evidence explaining why the failure is suspicious.
        evidence: String,
    },
}

impl LaneFailure {
    /// The tool that failed.
    #[must_use]
    pub fn tool(&self) -> &str {
        match self {
            Self::Infra { tool, .. }
            | Self::Tool { tool, .. }
            | Self::Resource { tool, .. }
            | Self::Suspicious { tool, .. } => tool,
        }
    }

    /// Whether this is an infrastructure issue (not a code problem).
    #[must_use]
    pub const fn is_infra(&self) -> bool {
        matches!(self, Self::Infra { .. })
    }

    /// Whether this is a resource constraint.
    #[must_use]
    pub const fn is_resource(&self) -> bool {
        matches!(self, Self::Resource { .. })
    }
}
