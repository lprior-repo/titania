use serde::{Deserialize, Deserializer, Serialize};

use crate::error::FailureError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProcessTermination {
    Exited { code: i32 },
    Signaled { signal: i32 },
    TimedOut,
    MemoryLimitExceeded,
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
    pub fn signaled(signal: i32) -> Result<Self, FailureError> {
        (signal > 0).then_some(Self::Signaled { signal }).ok_or(FailureError::InvalidSignal(signal))
    }

    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Exited { code } if code == 0)
    }

    #[must_use]
    pub const fn exit_code(self) -> Option<i32> {
        match self {
            Self::Exited { code } => Some(code),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneFailure {
    #[serde(rename = "InfraFailure")]
    Infra { tool: String, reason: String },
    #[serde(rename = "ToolFailure")]
    Tool { tool: String, termination: ProcessTermination },
    #[serde(rename = "ResourceFailure")]
    Resource { tool: String, limit: String },
    #[serde(rename = "SuspiciousFailure")]
    Suspicious { tool: String, evidence: String },
}

impl LaneFailure {
    #[must_use]
    pub fn tool(&self) -> &str {
        match self {
            Self::Infra { tool, .. }
            | Self::Tool { tool, .. }
            | Self::Resource { tool, .. }
            | Self::Suspicious { tool, .. } => tool,
        }
    }

    #[must_use]
    pub const fn is_infra(&self) -> bool {
        matches!(self, Self::Infra { .. })
    }

    #[must_use]
    pub const fn is_resource(&self) -> bool {
        matches!(self, Self::Resource { .. })
    }
}
