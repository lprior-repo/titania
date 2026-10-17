mod budget;
mod env_filter;
mod error;
mod execution;
mod output;
mod process;
mod reader;

use std::{borrow::Cow, time::Duration};

pub use budget::{CommandBudget, EnvPolicy};
pub use error::{LaneError, OutputStream};
pub use output::CommandOutput;
use smallvec::SmallVec;
use titania_core::TargetProject;

const TERMINATION_GRACE: Duration = Duration::from_secs(1);

type EnvPair<'a> = (Cow<'a, str>, Cow<'a, str>);

#[derive(Debug)]
pub struct CommandIn<'a> {
    cwd: &'a TargetProject,
    program: Cow<'a, str>,
    args: SmallVec<[Cow<'a, str>; 8]>,
    env: SmallVec<[EnvPair<'a>; 4]>,
    env_remove: SmallVec<[Cow<'a, str>; 4]>,
    env_policy: EnvPolicy,
    budget: CommandBudget,
}

impl<'a> CommandIn<'a> {
    pub fn new(cwd: &'a TargetProject, program: &'a str) -> Result<Self, LaneError> {
        validate_program(program)?;
        Ok(Self {
            cwd,
            program: Cow::Borrowed(program),
            args: SmallVec::new(),
            env: SmallVec::new(),
            env_remove: SmallVec::new(),
            env_policy: EnvPolicy::Clear,
            budget: CommandBudget::default(),
        })
    }

    pub fn arg(&mut self, a: &'a str) -> &mut Self {
        self.args.push(Cow::Borrowed(a));
        self
    }

    pub fn args(&mut self, as_: &'a [&'a str]) -> &mut Self {
        self.args.extend(as_.iter().map(|s| Cow::Borrowed(*s)));
        self
    }

    pub fn args_strings(&mut self, as_: &[String]) -> &mut Self {
        self.args.extend(as_.iter().map(|s| Cow::Owned(s.clone())));
        self
    }

    pub fn env(&mut self, k: &'a str, v: &'a str) -> &mut Self {
        self.env.push((Cow::Borrowed(k), Cow::Borrowed(v)));
        self
    }

    pub fn env_remove(&mut self, k: &'a str) -> &mut Self {
        self.env_remove.push(Cow::Borrowed(k));
        self
    }

    pub const fn inherit_env(&mut self) -> &mut Self {
        self.env_policy = EnvPolicy::Inherit;
        self
    }

    pub const fn budget(&mut self, budget: CommandBudget) -> &mut Self {
        self.budget = budget;
        self
    }
}

fn validate_program(program: &str) -> Result<(), LaneError> {
    if program.is_empty() {
        Err(LaneError::EmptyProgram)
    } else if program.contains('\0') {
        Err(LaneError::InvalidProgram)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn mock_output(
    exit_code: i32,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<CommandOutput, std::io::Error> {
    let status = std::process::Command::new(if exit_code == 0 { "true" } else { "false" })
        .output()
        .map(|o| o.status)?;
    Ok(CommandOutput::new(status, stdout.to_vec(), stderr.to_vec(), String::from("test")))
}
