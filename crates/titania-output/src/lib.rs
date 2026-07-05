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
    /// A syntactically valid rule ID is absent from the static catalog.
    UnknownRule {
        /// Requested rule identifier.
        rule_id: String,
    },
}

impl OutputError {
    /// Build an unavailable-component error.
    #[must_use]
    pub const fn component_unavailable(component: OutputComponent) -> Self {
        Self::ComponentUnavailable { component }
    }

    /// Build an unknown-rule error.
    #[must_use]
    pub fn unknown_rule(rule_id: &str) -> Self {
        Self::UnknownRule { rule_id: rule_id.to_owned() }
    }
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ComponentUnavailable { component } => component.as_str(),
            Self::UnknownRule { rule_id } => return write!(f, "unknown rule ID: {rule_id}"),
        };
        write!(f, "{message} output is not implemented yet")
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

// Rule explanation output contract — replaced by the file-based module
// in `explain.rs` which owns catalog data and test scaffolding.
pub mod explain;
