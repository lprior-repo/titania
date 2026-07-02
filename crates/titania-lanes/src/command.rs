//! `CommandIn`: single chokepoint for shelling out from a lane.
//!
//! Every subprocess a lane launches MUST go through [`CommandIn`] so
//! that the working directory, environment policy, execution budget,
//! output budget, exit-code handling, and UTF-8 decoding behavior are
//! explicit and typed.

mod budget;
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

/// A typed builder for `std::process::Command` rooted at a target project.
type EnvPair<'a> = (Cow<'a, str>, Cow<'a, str>);

/// Typed subprocess builder rooted at a [`TargetProject`].
///
/// Every lane shells out through `CommandIn` so the working directory,
/// environment policy, execution/output budgets, and exit-code handling
/// are explicit and uniform. See the module docs for the full contract.
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
    /// Create a new `CommandIn` for `program` inside the target project.
    ///
    /// # Errors
    /// [`LaneError::EmptyProgram`] if `program` is empty;
    /// [`LaneError::InvalidProgram`] if it contains NUL bytes.
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

    /// Append a single argument. Returns `&mut self` for chaining.
    pub fn arg(&mut self, a: &'a str) -> &mut Self {
        self.args.push(Cow::Borrowed(a));
        self
    }

    /// Append multiple arguments. Returns `&mut self` for chaining.
    pub fn args(&mut self, as_: &'a [&'a str]) -> &mut Self {
        self.args.extend(as_.iter().map(|s| Cow::Borrowed(*s)));
        self
    }

    /// Append multiple owned-string arguments from a slice of `String`,
    /// storing owned copies so the borrow is not tied to the
    /// [`CommandIn`] lifetime parameter.
    pub fn args_strings(&mut self, as_: &[String]) -> &mut Self {
        self.args.extend(as_.iter().map(|s| Cow::Owned(s.clone())));
        self
    }

    /// Set an environment variable in the spawned process.
    pub fn env(&mut self, k: &'a str, v: &'a str) -> &mut Self {
        self.env.push((Cow::Borrowed(k), Cow::Borrowed(v)));
        self
    }

    /// Remove an inherited environment variable from the spawned process.
    pub fn env_remove(&mut self, k: &'a str) -> &mut Self {
        self.env_remove.push(Cow::Borrowed(k));
        self
    }

    /// Explicitly inherit the parent environment.
    pub const fn inherit_env(&mut self) -> &mut Self {
        self.env_policy = EnvPolicy::Inherit;
        self
    }

    /// Replace the default execution/output budget.
    pub const fn budget(&mut self, budget: CommandBudget) -> &mut Self {
        self.budget = budget;
        self
    }
}

/// Validate that a command program can be passed to `std::process::Command`.
///
/// # Errors
/// Returns [`LaneError::EmptyProgram`] if `program` is empty, or
/// [`LaneError::InvalidProgram`] if it contains a NUL byte.
fn validate_program(program: &str) -> Result<(), LaneError> {
    if program.is_empty() {
        Err(LaneError::EmptyProgram)
    } else if program.contains('\0') {
        Err(LaneError::InvalidProgram)
    } else {
        Ok(())
    }
}
