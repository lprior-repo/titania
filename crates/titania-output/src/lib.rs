#![warn(missing_docs)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]
#![forbid(unsafe_code)]

//! Shared typed output contracts for titania doctor and explain commands.
//!
//! This crate owns the output data types used by the `titania-check` CLI's
//! `--emit json` and `--emit human` modes. It also provides a minimal
//! `OutputError` type for reporting unavailable output components.

use thiserror::Error;

/// Human-readable label for a single output component.
///
/// Used as the component identifier in doctor reports and error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputComponent {
    /// Doctor report output.
    Doctor,
    /// Explain output.
    Explain,
}

impl OutputComponent {
    /// Return the human-readable label.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Explain => "explain",
        }
    }
}

/// Error returned when an output component cannot produce a report.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OutputError {
    /// The component is a declared contract, but its implementation bead has not landed.
    #[error("output is not implemented yet for {component:?}")]
    ComponentUnavailable {
        /// Component that is intentionally unavailable.
        component: OutputComponent,
    },
    /// A syntactically valid rule ID is absent from the static catalog.
    #[error("unknown rule ID: {rule_id}")]
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

/// Doctor tool/version diagnostic domain model.
pub mod doctor;
// Rule explanation output contract — replaced by the file-based module
// in `explain.rs` which owns catalog data and test scaffolding.
pub mod explain;
