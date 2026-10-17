mod parse;

use std::{ffi::OsString, path::PathBuf};

use titania_core::{GateScope, Lane};

type ValueTail<'a> = (&'a str, &'a [String]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Check(CheckOptions),
    RunLane { lane: Lane },
    Aggregate(AggregateOptions),
    Doctor(DoctorOptions),
    Explain { rule_id: String },
    SetupHermetic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOptions {
    pub scope: GateScope,
    emit: EmitFormat,
    out: Option<PathBuf>,
}
impl CheckOptions {
    #[must_use]
    pub const fn emit(&self) -> EmitFormat {
        self.emit
    }

    #[must_use]
    pub const fn out(&self) -> Option<&PathBuf> {
        self.out.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateOptions {
    pub scope: GateScope,
    emit: EmitFormat,
    out: Option<PathBuf>,
}
impl AggregateOptions {
    #[must_use]
    pub const fn emit(&self) -> EmitFormat {
        self.emit
    }

    #[must_use]
    pub const fn out(&self) -> Option<&PathBuf> {
        self.out.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoctorOptions {
    pub scope: GateScope,
    emit: EmitFormat,
}

impl DoctorOptions {
    #[must_use]
    pub const fn emit(self) -> EmitFormat {
        self.emit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    HelpRequested(String),
    VersionRequested(String),
    NonUtf8Argument(String),
    UnknownSubcommand(String),
    UnknownFlag(String),
    MissingValue(&'static str),
    UnknownScope(String),
    UnknownEmit(String),
    UnknownLane(String),
    MissingLaneName,
    ExtraArgument { command: &'static str, value: String },
    MissingRuleId,
    InvalidRuleId { value: String, reason: String },
    AggregateScopeRequired,
}

impl Cli {
    pub fn parse_from_os<I>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = OsString>,
    {
        parse::parse_from_os(args)
    }
}

impl CliError {
    #[must_use]
    pub fn diagnostic(&self) -> String {
        match self {
            Self::HelpRequested(text) | Self::VersionRequested(text) => text.clone(),
            Self::NonUtf8Argument(value) => diagnostic_non_utf8(value),
            Self::UnknownSubcommand(value) => format!("InputError: unknown subcommand '{value}'"),
            Self::UnknownFlag(value) => format!("InputError: unknown flag '{value}'"),
            Self::MissingValue(flag) => format!("InputError: missing value for {flag}"),
            Self::UnknownScope(value) => format!(
                "InputError: unknown scope '{value}'; expected one of: edit, prepush, release, full"
            ),
            Self::UnknownEmit(value) => format!("InputError: unknown emit format '{value}'"),
            Self::UnknownLane(value) => format!(
                "InputError: unknown lane '{value}'; expected one of: fmt, compile, clippy, ast-grep, dylint, panic-scan, policy-scan, test, deny, build, kani, mutants"
            ),
            Self::MissingLaneName => String::from("InputError: run-lane requires a lane name"),
            Self::ExtraArgument { command, value } => diagnostic_extra_arg(command, value),
            Self::MissingRuleId => String::from("InputError: explain requires a rule id"),
            Self::InvalidRuleId { value, reason } => diagnostic_rule_id(value, reason),
            Self::AggregateScopeRequired => diagnostic_aggregate_scope_required(),
        }
    }

    #[must_use]
    pub const fn is_help(&self) -> bool {
        matches!(self, Self::HelpRequested(_))
    }

    #[must_use]
    pub fn help_text(&self) -> Option<&str> {
        match self {
            Self::HelpRequested(text) => Some(text.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_version(&self) -> bool {
        matches!(self, Self::VersionRequested(_))
    }

    #[must_use]
    pub fn version_text(&self) -> Option<&str> {
        match self {
            Self::VersionRequested(text) => Some(text.as_str()),
            _ => None,
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
    String::from("InputError: aggregate requires --scope <edit|prepush|release|full>")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AggregateState {
    scope: Option<GateScope>,
    emit: EmitFormat,
    out: Option<PathBuf>,
}

impl Default for AggregateState {
    fn default() -> Self {
        Self { scope: None, emit: EmitFormat::Json, out: None }
    }
}
impl Default for CheckOptions {
    fn default() -> Self {
        Self { scope: GateScope::Edit, emit: EmitFormat::Json, out: None }
    }
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self { scope: GateScope::Edit, emit: EmitFormat::Human }
    }
}

#[must_use]
pub fn version_string() -> String {
    format!(
        "titania-check {version} (rev {sha}, workspace={workspace})",
        version = env!("CARGO_PKG_VERSION"),
        sha = crate::version::BUILD_GIT_SHA,
        workspace = crate::version::WORKSPACE_NAME,
    )
}

pub(crate) const USAGE: &str = "\
titania-check — Titania quality gate CLI

USAGE:
    titania-check [OPTIONS]                 Run scoped quality lanes via Moon (default: check).
    titania-check check [OPTIONS]           Run scoped quality lanes via Moon.
    titania-check run-lane <lane-name>      Run a single lane and write findings artifact.
    titania-check aggregate --scope <s> [OPTIONS]
                                            Read existing lane artifacts and emit a report.
    titania-check doctor [OPTIONS]          Report required tools and versions for a scope.
    titania-check explain <rule-id>         Print rule description and metadata.
    titania-check setup-hermetic            Create hermetic CARGO_HOME / RUSTUP_HOME symlinks.

OPTIONS:
    --scope <edit|prepush|release|full>     Gate scope (default: edit).
    --emit <json>                           Check/aggregate output (default: json); doctor accepts json|human (default: human).
    --out <path>                            Write report to file instead of stdout.
    --version, -V                           Print version and git SHA, then exit 0.

LANES (run-lane):
    fmt, compile, clippy, ast-grep, dylint, panic-scan, policy-scan,
    test, deny, build, kani, mutants

EXIT CODES:
    0  Pass
    1  Reject (code findings and/or gate failures)
    2  PolicyError
    3  InputError
    >=4 Internal error

See `titania-check explain <rule-id>` for rule details and v1-spec.md for the
full specification.";
