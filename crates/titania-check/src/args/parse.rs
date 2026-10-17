use std::{ffi::OsString, path::PathBuf};

use titania_core::{GateScope, Lane};

use super::{
    AggregateOptions, AggregateState, CheckOptions, Cli, CliError, Command, DoctorOptions,
    EmitFormat, USAGE, ValueTail,
};

pub(super) fn parse_from_os<I>(args: I) -> Result<Cli, CliError>
where
    I: IntoIterator<Item = OsString>,
{
    collect_utf8(args).and_then(|strings| parse_command(&strings))
}

fn collect_utf8<I: IntoIterator<Item = OsString>>(args: I) -> Result<Vec<String>, CliError> {
    args.into_iter()
        .map(|arg| {
            arg.into_string().map_err(|error| CliError::NonUtf8Argument(format!("{error:?}")))
        })
        .collect()
}

fn parse_command(args: &[String]) -> Result<Cli, CliError> {
    let Some((head, tail)) = args.split_first() else {
        return Ok(Cli { command: Command::Check(CheckOptions::default()) });
    };
    if is_top_level_help(head) {
        return help_request();
    }
    if is_top_level_version(head) {
        return version_request();
    }
    parse_subcommand(head, tail, args)
}

fn parse_subcommand(head: &str, tail: &[String], full_args: &[String]) -> Result<Cli, CliError> {
    match head {
        "check" => parse_check(tail),
        "run-lane" => parse_run_lane(tail),
        "aggregate" => parse_aggregate(tail),
        "doctor" => parse_doctor(tail),
        "explain" => parse_explain(tail),
        "setup-hermetic" => parse_setup_hermetic(tail),
        candidate if candidate.starts_with('-') => parse_check(full_args),
        _ => Err(CliError::UnknownSubcommand(head.to_owned())),
    }
}

fn is_top_level_help(token: &str) -> bool {
    matches!(token, "--help" | "-h" | "help")
}

fn is_help_flag(token: &str) -> bool {
    matches!(token, "--help" | "-h")
}

fn help_request() -> Result<Cli, CliError> {
    Err(help_request_error())
}

fn help_request_error() -> CliError {
    CliError::HelpRequested(String::from(USAGE))
}

fn is_top_level_version(token: &str) -> bool {
    matches!(token, "--version" | "-V")
}

fn version_request_error() -> CliError {
    CliError::VersionRequested(super::version_string())
}

fn version_request() -> Result<Cli, CliError> {
    Err(version_request_error())
}

fn parse_check(args: &[String]) -> Result<Cli, CliError> {
    let options = parse_check_options(args, CheckOptions::default())?;
    Ok(Cli { command: Command::Check(options) })
}

fn parse_check_options(args: &[String], options: CheckOptions) -> Result<CheckOptions, CliError> {
    match args.split_first() {
        None => Ok(options),
        Some((flag, _)) if is_help_flag(flag) => Err(help_request_error()),
        Some((flag, tail)) if flag == "--scope" => parse_check_scope(tail, options),
        Some((flag, tail)) if flag == "--emit" => parse_check_emit(tail, options),
        Some((flag, tail)) if flag == "--out" => parse_check_out(tail, &options),
        Some((flag, _)) if flag.starts_with('-') => Err(CliError::UnknownFlag(flag.clone())),
        Some((value, _)) => Err(CliError::UnknownSubcommand(value.clone())),
    }
}

fn parse_check_scope(args: &[String], options: CheckOptions) -> Result<CheckOptions, CliError> {
    let (value, rest) = take_value("--scope", args)?;
    let scope = parse_scope(value)?;
    parse_check_options(rest, CheckOptions { scope, emit: options.emit, out: options.out })
}

fn parse_check_emit(args: &[String], options: CheckOptions) -> Result<CheckOptions, CliError> {
    let (value, rest) = take_value("--emit", args)?;
    let emit = parse_primary_emit(value)?;
    parse_check_options(rest, CheckOptions { scope: options.scope, emit, out: options.out })
}

fn parse_check_out(args: &[String], options: &CheckOptions) -> Result<CheckOptions, CliError> {
    let (value, rest) = take_value("--out", args)?;
    parse_check_options(
        rest,
        CheckOptions { scope: options.scope, emit: options.emit, out: Some(PathBuf::from(value)) },
    )
}

fn parse_aggregate(args: &[String]) -> Result<Cli, CliError> {
    let state = parse_aggregate_options(args, AggregateState::default())?;
    match state.scope {
        Some(scope) => Ok(Cli {
            command: Command::Aggregate(AggregateOptions {
                scope,
                emit: state.emit,
                out: state.out,
            }),
        }),
        None => Err(CliError::AggregateScopeRequired),
    }
}

fn parse_aggregate_options(
    args: &[String],
    state: AggregateState,
) -> Result<AggregateState, CliError> {
    match args.split_first() {
        None => Ok(state),
        Some((flag, _)) if is_help_flag(flag) => Err(help_request_error()),
        Some((flag, tail)) if flag == "--scope" => parse_aggregate_scope(tail, state),
        Some((flag, tail)) if flag == "--emit" => parse_aggregate_emit(tail, state),
        Some((flag, tail)) if flag == "--out" => parse_aggregate_out(tail, &state),
        Some((flag, _)) if flag.starts_with('-') => Err(CliError::UnknownFlag(flag.clone())),
        Some((value, _)) => {
            Err(CliError::ExtraArgument { command: "aggregate", value: value.clone() })
        }
    }
}

fn parse_aggregate_scope(
    args: &[String],
    state: AggregateState,
) -> Result<AggregateState, CliError> {
    let (value, rest) = take_value("--scope", args)?;
    let scope = parse_scope(value)?;
    parse_aggregate_options(
        rest,
        AggregateState { scope: Some(scope), emit: state.emit, out: state.out },
    )
}

fn parse_aggregate_emit(
    args: &[String],
    state: AggregateState,
) -> Result<AggregateState, CliError> {
    let (value, rest) = take_value("--emit", args)?;
    let emit = parse_primary_emit(value)?;
    parse_aggregate_options(rest, AggregateState { scope: state.scope, emit, out: state.out })
}

fn parse_aggregate_out(
    args: &[String],
    state: &AggregateState,
) -> Result<AggregateState, CliError> {
    let (value, rest) = take_value("--out", args)?;
    parse_aggregate_options(
        rest,
        AggregateState { scope: state.scope, emit: state.emit, out: Some(PathBuf::from(value)) },
    )
}

fn parse_doctor(args: &[String]) -> Result<Cli, CliError> {
    let options = parse_doctor_options(args, DoctorOptions::default())?;
    Ok(Cli { command: Command::Doctor(options) })
}

fn parse_doctor_options(
    args: &[String],
    options: DoctorOptions,
) -> Result<DoctorOptions, CliError> {
    match args.split_first() {
        None => Ok(options),
        Some((flag, _)) if is_help_flag(flag) => Err(help_request_error()),
        Some((flag, tail)) if flag == "--scope" => parse_doctor_scope(tail, options),
        Some((flag, tail)) if flag == "--emit" => parse_doctor_emit(tail, options),
        Some((flag, _)) if flag.starts_with('-') => Err(CliError::UnknownFlag(flag.clone())),
        Some((value, _)) => {
            Err(CliError::ExtraArgument { command: "doctor", value: value.clone() })
        }
    }
}

fn parse_doctor_scope(args: &[String], options: DoctorOptions) -> Result<DoctorOptions, CliError> {
    let (value, rest) = take_value("--scope", args)?;
    let scope = parse_scope(value)?;
    parse_doctor_options(rest, DoctorOptions { scope, emit: options.emit })
}

fn parse_doctor_emit(args: &[String], options: DoctorOptions) -> Result<DoctorOptions, CliError> {
    let (value, rest) = take_value("--emit", args)?;
    let emit = parse_emit(value)?;
    parse_doctor_options(rest, DoctorOptions { scope: options.scope, emit })
}

fn parse_explain(args: &[String]) -> Result<Cli, CliError> {
    if args.is_empty() {
        return Err(CliError::MissingRuleId);
    }
    let first = args.first().ok_or(CliError::MissingRuleId)?;
    if is_help_flag(first) {
        return help_request();
    }
    let rest: &[String] = match args.get(1..) {
        Some(s) => s,
        None => &[],
    };
    if !rest.is_empty() {
        return extra_arg("explain", rest);
    }
    Ok(Cli { command: Command::Explain { rule_id: first.to_owned() } })
}

fn parse_setup_hermetic(args: &[String]) -> Result<Cli, CliError> {
    match args.split_first() {
        None => Ok(Cli { command: Command::SetupHermetic }),
        Some((flag, _)) if is_help_flag(flag) => help_request(),
        Some((value, _)) => {
            Err(CliError::ExtraArgument { command: "setup-hermetic", value: value.clone() })
        }
    }
}

fn parse_run_lane(args: &[String]) -> Result<Cli, CliError> {
    match args.split_first() {
        None => Err(CliError::MissingLaneName),
        Some((flag, _)) if is_help_flag(flag) => help_request(),
        Some((lane, [])) => parse_lane(lane).map(|lane| Cli { command: Command::RunLane { lane } }),
        Some((_, tail)) => extra_arg("run-lane", tail),
    }
}

fn extra_arg(command: &'static str, args: &[String]) -> Result<Cli, CliError> {
    match args.split_first() {
        Some((value, _)) => Err(CliError::ExtraArgument { command, value: value.clone() }),
        None => Err(CliError::MissingValue(command)),
    }
}

fn parse_scope(value: &str) -> Result<GateScope, CliError> {
    value.parse::<GateScope>().map_err(|_err| CliError::UnknownScope(value.to_owned()))
}

fn parse_emit(value: &str) -> Result<EmitFormat, CliError> {
    match value {
        "human" => Ok(EmitFormat::Human),
        "json" => Ok(EmitFormat::Json),
        _ => Err(CliError::UnknownEmit(value.to_owned())),
    }
}
fn parse_primary_emit(value: &str) -> Result<EmitFormat, CliError> {
    match value {
        "json" => Ok(EmitFormat::Json),
        _ => Err(CliError::UnknownEmit(value.to_owned())),
    }
}

fn parse_lane(value: &str) -> Result<Lane, CliError> {
    match value {
        "fmt" => Ok(Lane::Fmt),
        "compile" => Ok(Lane::Compile),
        "clippy" => Ok(Lane::Clippy),
        "ast-grep" => Ok(Lane::AstGrep),
        "dylint" => Ok(Lane::Dylint),
        "panic-scan" => Ok(Lane::PanicScan),
        "policy-scan" => Ok(Lane::PolicyScan),
        "test" => Ok(Lane::Test),
        "deny" => Ok(Lane::Deny),
        "build" => Ok(Lane::Build),
        "kani" => Ok(Lane::Kani),
        "mutants" => Ok(Lane::Mutants),
        _ => Err(CliError::UnknownLane(value.to_owned())),
    }
}

fn take_value<'a>(flag: &'static str, args: &'a [String]) -> Result<ValueTail<'a>, CliError> {
    args.split_first()
        .map(|(value, rest)| (value.as_str(), rest))
        .ok_or(CliError::MissingValue(flag))
}
