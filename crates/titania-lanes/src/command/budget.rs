use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_STDOUT: usize = 1024 * 1024;
const DEFAULT_MAX_STDERR: usize = 1024 * 1024;

/// Environment policy for a spawned command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvPolicy {
    /// Clear the process environment, then apply explicitly supplied env
    /// pairs. This is the default for deterministic target judgment.
    Clear,
    /// Inherit the parent process environment, then apply explicitly
    /// supplied env pairs.
    Inherit,
}

/// Bounded execution policy for a spawned command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandBudget {
    /// Maximum wall-clock execution time for the spawned command.
    pub timeout: Duration,
    /// Maximum captured stdout size, in bytes.
    pub max_stdout: usize,
    /// Maximum captured stderr size, in bytes.
    pub max_stderr: usize,
}

impl Default for CommandBudget {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_stdout: DEFAULT_MAX_STDOUT,
            max_stderr: DEFAULT_MAX_STDERR,
        }
    }
}
