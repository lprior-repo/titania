//! Policy and input diagnostics.
//!
//! Diagnostics are emitted by the policy loader and CLI parser before any
//! lane runs, describing configuration or invocation problems.

use serde::{Deserialize, Serialize};

use crate::workspace_path::WorkspacePath;

/// Severity of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    /// The diagnostic blocks execution.
    Error,
    /// The diagnostic is a warning — execution continues.
    Warning,
}

/// A diagnostic produced by policy loading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDiagnostic {
    /// Human-readable diagnostic message.
    pub message: String,
    /// Policy file associated with the diagnostic, when known.
    pub file: Option<WorkspacePath>,
    /// Severity controlling whether execution may continue.
    pub severity: DiagnosticSeverity,
}

impl PolicyDiagnostic {
    /// Construct a new policy diagnostic.
    #[must_use]
    pub const fn new(
        message: String,
        file: Option<WorkspacePath>,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self { message, file, severity }
    }

    /// Whether this is an error-level diagnostic.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    /// Whether this is a warning-level diagnostic.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.severity == DiagnosticSeverity::Warning
    }
}

/// A diagnostic produced by CLI parsing or input validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputDiagnostic {
    /// Human-readable diagnostic message.
    pub message: String,
    /// Tool or subcommand associated with the diagnostic, when known.
    pub tool: Option<String>,
    /// Severity controlling whether execution may continue.
    pub severity: DiagnosticSeverity,
}

impl InputDiagnostic {
    /// Construct a new input diagnostic.
    #[must_use]
    pub const fn new(message: String, tool: Option<String>, severity: DiagnosticSeverity) -> Self {
        Self { message, tool, severity }
    }

    /// Whether this is an error-level diagnostic.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    /// Whether this is a warning-level diagnostic.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.severity == DiagnosticSeverity::Warning
    }
}
