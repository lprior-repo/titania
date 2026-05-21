#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XtaskCommandError {
    UnknownCommand {
        command: String,
    },
    MissingRequiredInput {
        command: String,
        input: String,
    },
    InvalidInput {
        command: String,
        input: String,
        reason: String,
    },
    OutputRenderFailed {
        command: String,
        reason: String,
    },
    DependencyBoundaryViolation {
        crate_name: String,
        dependency: String,
    },
    Unavailable {
        command: String,
        reason: String,
    },
    InternalInvariantViolation {
        invariant: String,
    },
}

impl std::fmt::Display for XtaskCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for XtaskCommandError {}
