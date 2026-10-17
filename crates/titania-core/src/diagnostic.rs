use serde::{Deserialize, Serialize};

use crate::workspace_path::WorkspacePath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDiagnostic {
    pub message: String,
    pub file: Option<WorkspacePath>,
    pub severity: DiagnosticSeverity,
}

impl PolicyDiagnostic {
    #[must_use]
    pub const fn new(
        message: String,
        file: Option<WorkspacePath>,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self { message, file, severity }
    }

    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.severity == DiagnosticSeverity::Warning
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputDiagnostic {
    pub message: String,
    pub tool: Option<String>,
    pub severity: DiagnosticSeverity,
}

impl InputDiagnostic {
    #[must_use]
    pub const fn new(message: String, tool: Option<String>, severity: DiagnosticSeverity) -> Self {
        Self { message, tool, severity }
    }

    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.severity == DiagnosticSeverity::Warning
    }
}
