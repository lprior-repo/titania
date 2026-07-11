//! Lane failure classification and process termination reasons.
//!
//! When a lane does not produce a clean or finding outcome, the failure is
//! categorized so that the aggregator can decide how to report it.

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::FailureError;

/// Why a process terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProcessTermination {
    /// Process exited with a specific exit code.
    Exited {
        /// Process exit code reported by the operating system.
        code: i32,
    },
    /// Process was terminated by a signal (Unix only).
    ///
    /// Signal numbers are positive integers. Values 1–31 are the standard
    /// POSIX signals (SIGHUP through SIGSYS). Values >= 32 are real-time
    /// signals (SIGRTMIN..SIGRTMAX on glibc, e.g. 34 = SIGRTMIN+2).
    /// Non-positive values are rejected because no Unix signal has number
    /// zero or negative — `0` is reserved for "no signal" in `waitpid(2)`
    /// status words and negative values are kernel-internal sentinels.
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

#[derive(Debug, Deserialize)]
enum ProcessTerminationWire {
    Exited { code: i32 },
    Signaled { signal: i32 },
    TimedOut,
    MemoryLimitExceeded,
    SpawnFailed,
}

impl<'de> Deserialize<'de> for ProcessTermination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProcessTerminationWire::deserialize(deserializer)?;
        let parse_signal = |signal: i32| Self::signaled(signal).map_err(serde::de::Error::custom);
        match wire {
            ProcessTerminationWire::Exited { code } => Ok(Self::Exited { code }),
            ProcessTerminationWire::Signaled { signal } => parse_signal(signal),
            ProcessTerminationWire::TimedOut => Ok(Self::TimedOut),
            ProcessTerminationWire::MemoryLimitExceeded => Ok(Self::MemoryLimitExceeded),
            ProcessTerminationWire::SpawnFailed => Ok(Self::SpawnFailed),
        }
    }
}

impl ProcessTermination {
    /// Construct a signal-based termination.
    ///
    /// Accepts any positive Unix signal number, including real-time signals
    /// (>= 32 on glibc, e.g. 34 = SIGRTMIN+2). Non-positive values are
    /// rejected — signal `0` is reserved and negative values are not
    /// meaningful signal identifiers.
    ///
    /// # Errors
    /// - [`FailureError::InvalidSignal`] if the signal is not positive.
    pub fn signaled(signal: i32) -> Result<Self, FailureError> {
        (signal > 0).then_some(Self::Signaled { signal }).ok_or(FailureError::InvalidSignal(signal))
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
pub enum LaneFailure {
    /// Infrastructure problem — tool binary missing, ABI mismatch, etc.
    #[serde(rename = "InfraFailure")]
    Infra {
        /// Tool that could not be invoked or trusted.
        tool: String,
        /// Stable reason explaining the infrastructure failure.
        reason: String,
    },
    /// Tool ran but terminated abnormally.
    #[serde(rename = "ToolFailure")]
    Tool {
        /// Tool that ran and returned an abnormal termination.
        tool: String,
        /// Termination status observed from the tool process.
        termination: ProcessTermination,
    },
    /// Resource constraint hit — memory limit, file descriptor limit, etc.
    #[serde(rename = "ResourceFailure")]
    Resource {
        /// Tool that exceeded a resource limit.
        tool: String,
        /// Resource limit that was exceeded.
        limit: String,
    },
    /// Tool failed but the cause is suspicious (e.g. intermittent,
    /// non-reproducible). The `evidence` field contains additional context.
    #[serde(rename = "SuspiciousFailure")]
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
