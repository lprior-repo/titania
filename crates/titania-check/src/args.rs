//! Argument parsing for the `titania-check` dispatch shell.

mod parse;

use std::{ffi::OsString, path::PathBuf};

use titania_core::{GateScope, Lane, RuleId};

type ValueTail<'a> = (&'a str, &'a [String]);

/// Parsed CLI invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    /// Top-level command selected by the invocation.
    pub command: Command,
}

/// Parsed top-level command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Default quality gate command.
    Check(CheckOptions),
    /// Single-lane dispatch command.
    RunLane {
        /// Lane selected for dispatch.
        lane: Lane,
    },
    /// Aggregate existing lane artifacts.
    Aggregate(AggregateOptions),
    /// Tool/version diagnostic command.
    Doctor(DoctorOptions),
    /// Rule explanation command.
    Explain {
        /// Rule identifier to explain.
        rule_id: RuleId,
    },
}

/// Parsed default-check options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOptions {
    /// Gate scope selected for the default check command.
    pub scope: GateScope,
    emit: EmitFormat,
    out: Option<PathBuf>,
}

/// Parsed `aggregate` options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateOptions {
    /// Gate scope whose lane artifacts are aggregated.
    pub scope: GateScope,
    emit: EmitFormat,
    out: Option<PathBuf>,
}

/// Parsed `doctor` options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoctorOptions {
    /// Gate scope whose tool requirements are reported.
    pub scope: GateScope,
    emit: EmitFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitFormat {
    Human,
    Json,
}

/// Input validation failure produced by CLI parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// An argument could not be converted to UTF-8.
    NonUtf8Argument(String),
    /// The first positional token is not a known command.
    UnknownSubcommand(String),
    /// A flag is not accepted by the selected command.
    UnknownFlag(String),
    /// A flag that requires a value was missing that value.
    MissingValue(&'static str),
    /// Scope value is not one of edit, prepush, or release.
    UnknownScope(String),
    /// Emit value is not one of the supported renderers.
    UnknownEmit(String),
    /// Lane name is not a v1 lane CLI name.
    UnknownLane(String),
    /// `run-lane` was invoked without a lane name.
    MissingLaneName,
    /// A command received an extra positional argument.
    ExtraArgument {
        /// Command that rejected the value.
        command: &'static str,
        /// Unexpected argument value.
        value: String,
    },
    /// `explain` was invoked without a rule id.
    MissingRuleId,
    /// `explain` received a syntactically invalid rule id.
    InvalidRuleId {
        /// Rejected rule-id text.
        value: String,
        /// Validation failure text.
        reason: String,
    },
    /// `aggregate` was invoked without its required scope.
    AggregateScopeRequired,
}

impl Cli {
    /// Parse CLI arguments after the binary name.
    ///
    /// # Errors
    ///
    /// Returns a [`CliError`] when an argument is not UTF-8, a command or flag
    /// is unknown, a required value is missing, or a domain value fails typed
    /// parsing.
    pub fn parse_from_os<I>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = OsString>,
    {
        parse::parse_from_os(args)
    }
}

impl CliError {
    /// Render this input error as a stable diagnostic line.
    #[must_use]
    pub fn diagnostic(&self) -> String {
        match self {
            Self::NonUtf8Argument(value) => diagnostic_non_utf8(value),
            Self::UnknownSubcommand(value) => format!("InputError: unknown subcommand '{value}'"),
            Self::UnknownFlag(value) => format!("InputError: unknown flag '{value}'"),
            Self::MissingValue(flag) => format!("InputError: missing value for {flag}"),
            Self::UnknownScope(value) => format!("InputError: unknown scope '{value}'"),
            Self::UnknownEmit(value) => format!("InputError: unknown emit format '{value}'"),
            Self::UnknownLane(value) => format!("InputError: unknown lane '{value}'"),
            Self::MissingLaneName => String::from("InputError: run-lane requires a lane name"),
            Self::ExtraArgument { command, value } => diagnostic_extra_arg(command, value),
            Self::MissingRuleId => String::from("InputError: explain requires a rule id"),
            Self::InvalidRuleId { value, reason } => diagnostic_rule_id(value, reason),
            Self::AggregateScopeRequired => diagnostic_aggregate_scope_required(),
        }
    }
}

fn diagnostic_non_utf8(value: &str) -> String {
    format!("InputError: argument is not valid UTF-8: {value}")
}

fn diagnostic_extra_arg(command: &str, value: &str) -> String {
    format!("InputError: unexpected argument '{value}' for {command}")
}

fn diagnostic_rule_id(value: &str, reason: &str) -> String {
    format!("InputError: invalid rule id '{value}': {reason}")
}

fn diagnostic_aggregate_scope_required() -> String {
    String::from("InputError: aggregate requires --scope <edit|prepush|release>")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AggregateState {
    scope: Option<GateScope>,
    emit: EmitFormat,
    out: Option<PathBuf>,
}

impl Default for AggregateState {
    fn default() -> Self {
        Self { scope: None, emit: EmitFormat::Human, out: None }
    }
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self { scope: GateScope::Edit, emit: EmitFormat::Human, out: None }
    }
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self { scope: GateScope::Edit, emit: EmitFormat::Human }
    }
}
