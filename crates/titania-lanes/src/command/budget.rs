use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
const DEFAULT_MAX_STDOUT: usize = 1024 * 1024;
const DEFAULT_MAX_STDERR: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvPolicy {
    Clear,
    Inherit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandBudget {
    pub timeout: Duration,
    pub max_stdout: usize,
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
