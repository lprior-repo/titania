//! Argument parsing for the `titania-check` dispatch shell.

mod parse;

use std::{ffi::OsString, path::PathBuf};

use titania_core::{GateScope, Lane};

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
        /// Raw rule identifier to explain. Strict [`titania_core::RuleId`]
        /// validation is intentionally skipped per spec §12.
        rule_id: String,
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
impl CheckOptions {
    /// The render format selected by the caller.
    #[must_use]
    pub const fn emit(&self) -> EmitFormat {
        self.emit
    }

    /// Optional output file path.
    #[must_use]
    pub const fn out(&self) -> Option<&PathBuf> {
        self.out.as_ref()
    }
}

/// Parsed `aggregate` options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateOptions {
    /// Gate scope whose lane artifacts are aggregated.
    pub scope: GateScope,
    emit: EmitFormat,
    out: Option<PathBuf>,
}
impl AggregateOptions {
    /// The render format selected by the caller.
    #[must_use]
    pub const fn emit(&self) -> EmitFormat {
        self.emit
    }

    /// Optional output file path.
    #[must_use]
    pub const fn out(&self) -> Option<&PathBuf> {
        self.out.as_ref()
    }
}

/// Parsed `doctor` options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoctorOptions {
    /// Gate scope whose tool requirements are reported.
    pub scope: GateScope,
    emit: EmitFormat,
}

impl DoctorOptions {
    /// The render format selected by the caller.
    #[must_use]
    pub const fn emit(self) -> EmitFormat {
        self.emit
    }
}

/// Render format for CLI output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitFormat {
    /// Human-readable table or text output.
    Human,
    /// Machine-readable JSON output.
    Json,
}

/// Input validation failure produced by CLI parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// Help was requested via `--help`, `-h`, or `help`. Carries the rendered
    /// usage text. This is a success path (exit 0 with the help on stdout),
    /// modeled as a `CliError` variant so that [`Cli::parse_from_os`] can
    /// return it through the existing `Result` channel without a separate
    /// outcome type.
    HelpRequested(String),
    /// Version was requested via `--version` or `-V`. Carries the rendered
    /// version string. Routed to stdout with exit code 0 by the dispatch
    /// shell.
    VersionRequested(String),
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
            Self::HelpRequested(text) | Self::VersionRequested(text) => text.clone(),
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

    /// Return `true` when this error carries a help/usage payload that must be
    /// routed to stdout with exit code 0 (instead of stderr with exit code 3).
    #[must_use]
    pub const fn is_help(&self) -> bool {
        matches!(self, Self::HelpRequested(_))
    }

    /// Borrow the help text when this error is [`Self::HelpRequested`].
    #[must_use]
    pub fn help_text(&self) -> Option<&str> {
        match self {
            Self::HelpRequested(text) => Some(text.as_str()),
            _ => None,
        }
    }

    /// Return `true` when this error carries a version payload.
    #[must_use]
    pub const fn is_version(&self) -> bool {
        matches!(self, Self::VersionRequested(_))
    }

    /// Borrow the version text when this error is [`Self::VersionRequested`].
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

/// Render the version banner printed for `--version` / `-V`.
///
/// Produces: `titania-check <CARGO_PKG_VERSION> (rev <GIT_SHA>, workspace=<WORKSPACE_NAME>)`.
#[must_use]
pub fn version_string() -> String {
    format!(
        "titania-check {version} (rev {sha}, workspace={workspace})",
        version = env!("CARGO_PKG_VERSION"),
        sha = crate::version::BUILD_GIT_SHA,
        workspace = crate::version::WORKSPACE_NAME,
    )
}

/// Concise top-level usage summary printed for `--help`, `-h`, and `help`.
///
/// Lists the five subcommands and the global flags (`--scope`, `--emit`, `--out`,
/// `--version`). Kept as a single `&str` so the help path performs no formatting
/// work and is trivially auditable.
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

OPTIONS:
    --scope <edit|prepush|release>          Gate scope (default: edit).
    --emit <json>                           Check/aggregate output (default: json); doctor accepts json|human (default: human).
    --out <path>                            Write report to file instead of stdout.
    --version, -V                           Print version and git SHA, then exit 0.

LANES (run-lane):
    fmt, compile, clippy, ast-grep, dylint, panic-scan, policy-scan,
    test, deny, build

EXIT CODES:
    0  Pass
    1  Reject (code findings and/or gate failures)
    2  PolicyError
    3  InputError
    >=4 Internal error

See `titania-check explain <rule-id>` for rule details and v1-spec.md for the
full specification.";
