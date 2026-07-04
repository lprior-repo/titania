//! Shared typed output contracts for Titania doctor and explain commands.
//!
//! This crate intentionally contains contracts only. Until the doctor and rule
//! catalog beads land, public entry points report typed unavailability instead
//! of pretending that a successful report exists.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![forbid(unsafe_code)]

use core::fmt;

/// Output component whose implementation is not linked yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputComponent {
    /// Tool/version doctor report producer.
    Doctor,
    /// Rule explanation catalog producer.
    ExplainCatalog,
}

impl OutputComponent {
    /// Stable component name for diagnostics and external reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::ExplainCatalog => "explain_catalog",
        }
    }
}

/// Error returned when an output component cannot produce a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputError {
    /// The component is a declared contract, but its implementation bead has not landed.
    ComponentUnavailable {
        /// Component that is intentionally unavailable.
        component: OutputComponent,
    },
}

impl OutputError {
    /// Build an unavailable-component error.
    #[must_use]
    pub const fn component_unavailable(component: OutputComponent) -> Self {
        Self::ComponentUnavailable { component }
    }
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let component = match self {
            Self::ComponentUnavailable { component } => component,
        };
        write!(f, "{} output is not implemented yet", component.as_str())
    }
}

impl std::error::Error for OutputError {}

/// Doctor output contract.
pub mod doctor {
    use crate::{OutputComponent, OutputError};

    /// Typed doctor report placeholder.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DoctorReport;

    /// Return the current doctor report.
    ///
    /// # Errors
    /// Always returns [`OutputError::ComponentUnavailable`] until the doctor
    /// implementation bead wires concrete tool/version checks.
    pub const fn report() -> Result<DoctorReport, OutputError> {
        Err(OutputError::component_unavailable(OutputComponent::Doctor))
    }
}

/// Rule explanation output contract.
pub mod explain {
    use crate::{OutputComponent, OutputError};

    /// Typed rule explanation placeholder.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RuleExplanation;

    /// Return a rule explanation catalog entry.
    ///
    /// # Errors
    /// Always returns [`OutputError::ComponentUnavailable`] until the rule
    /// catalog bead wires concrete catalog data.
    pub const fn explain_rule(_rule_id: &str) -> Result<RuleExplanation, OutputError> {
        Err(OutputError::component_unavailable(OutputComponent::ExplainCatalog))
    }
}
