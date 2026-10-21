#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]
#![forbid(unsafe_code)]

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputComponent {
    Doctor,
    Explain,
}

impl OutputComponent {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Explain => "explain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OutputError {
    #[error("output is not implemented yet for {component:?}")]
    ComponentUnavailable { component: OutputComponent },
    #[error("unknown rule ID: {rule_id}")]
    UnknownRule { rule_id: String },
}

impl OutputError {
    #[must_use]
    pub const fn component_unavailable(component: OutputComponent) -> Self {
        Self::ComponentUnavailable { component }
    }

    #[must_use]
    pub fn unknown_rule(rule_id: &str) -> Self {
        Self::UnknownRule { rule_id: rule_id.to_owned() }
    }
}

pub mod doctor;
pub mod explain;
