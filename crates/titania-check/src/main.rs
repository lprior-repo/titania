#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]

pub mod aggregate;
pub mod args;
pub mod doctor;
pub mod explain;
pub mod moon;
pub mod version;

use std::{env, io, io::Write, path::PathBuf, process::ExitCode};

use args::{Cli, Command};
use titania_core::{GateScope, Lane};
use titania_lanes::LaneExit;

const EXIT_PASS: u8 = 0;
const EXIT_REJECT: u8 = 1;
const EXIT_POLICY_ERROR: u8 = 2;
const EXIT_INPUT_ERROR: u8 = 3;
const EXIT_INTERNAL_ERROR: u8 = 4;

fn main() -> ExitCode {
    exit(&run(env::args_os().skip(1)))
}

fn run<I>(args: I) -> CliDisposition
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    match Cli::parse_from_os(args) {
        Ok(cli) => dispatch(cli),
        Err(error) if error.is_help() => CliDisposition::report(error.diagnostic(), EXIT_PASS),
        Err(error) if error.is_version() => CliDisposition::report(error.diagnostic(), EXIT_PASS),
        Err(error) => CliDisposition::input_error(error.diagnostic()),
    }
}

fn dispatch(cli: Cli) -> CliDisposition {
    match cli.command {
        Command::Check(options) => check_with_moon(&options),
        Command::Aggregate(options) => {
            aggregate_with_opts(options.scope, options.emit(), options.out())
        }
        Command::RunLane { lane } => run_lane(lane),
        Command::Doctor(options) => doctor_scope(options.scope, options.emit()),
        Command::Explain { rule_id } => explain_rule(&rule_id),
        Command::SetupHermetic => setup_hermetic(),
    }
}

fn check_with_moon(options: &args::CheckOptions) -> CliDisposition {
    let scope = options.scope;
    let emit = options.emit();
    let out = options.out();
    let target_root = match env::current_dir() {
        Ok(root) => root,
        Err(error) => return current_dir_error(&error),
    };
    if let Err(error) = clear_scope_outputs(&target_root, scope) {
        return CliDisposition::internal_error(format!(
            "InternalError: cannot clear stale lane artifacts: {error}"
        ));
    }
    let moon_bin = moon::binary_path();
    let tasks = moon::tasks_for_scope(scope);
    let hermetic = moon::HermeticEnv::detect(&target_root);
    match moon::spawn_all(&moon_bin, &tasks, &hermetic) {
        Ok(()) => aggregate_after_moon(&target_root, scope, emit, out, &hermetic),
        Err(moon::MoonSpawnError::NotFound) => {
            CliDisposition::input_error(format!("InputError: {}", moon::MISSING_INSTALL_HINT))
        }
        Err(moon::MoonSpawnError::TimedOut { seconds }) => CliDisposition::internal_error(format!(
            "InternalError: moon spawn exceeded wallclock timeout ({seconds}s)"
        )),
        Err(moon::MoonSpawnError::Failed(error)) => {
            CliDisposition::internal_error(format!("InternalError: moon spawn failed: {error}"))
        }
    }
}

fn setup_hermetic() -> CliDisposition {
    let root = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            return CliDisposition::internal_error(format!(
                "InternalError: cannot read current directory: {error}"
            ));
        }
    };

    let hermetic_dir = root.join(".titania").join("hermetic");
    if let Err(error) = std::fs::create_dir_all(&hermetic_dir) {
        return CliDisposition::internal_error(format!(
            "InternalError: cannot create hermetic dir: {error}"
        ));
    }
    let cargo_link = hermetic_dir.join("cargo-home");
    let rustup_link = hermetic_dir.join("rustup-home");

    let real_cargo_home = match resolve_real_home("CARGO_HOME", ".cargo") {
        Ok(path) => path,
        Err(error) => {
            return CliDisposition::internal_error(format!(
                "InternalError: cannot resolve CARGO_HOME: {error}"
            ));
        }
    };
    let real_rustup_home = match resolve_real_home("RUSTUP_HOME", ".rustup") {
        Ok(path) => path,
        Err(error) => {
            return CliDisposition::internal_error(format!(
                "InternalError: cannot resolve RUSTUP_HOME: {error}"
            ));
        }
    };

    if let Err(error) = link_if_needed(&real_cargo_home, &cargo_link) {
        return CliDisposition::internal_error(format!(
            "InternalError: cannot link cargo-home: {error}"
        ));
    }
    if let Err(error) = link_if_needed(&real_rustup_home, &rustup_link) {
        return CliDisposition::internal_error(format!(
            "InternalError: cannot link rustup-home: {error}"
        ));
    }

    let output = format!(
        "export CARGO_HOME=\"{}\"\nexport RUSTUP_HOME=\"{}\"",
        real_cargo_home.display(),
        real_rustup_home.display()
    );
    CliDisposition::report(output, EXIT_PASS)
}

fn resolve_real_home(var: &str, suffix: &str) -> Result<PathBuf, io::Error> {
    let raw = match env::var(var) {
        Ok(value) => PathBuf::from(value),
        Err(_) => home_from_env()?.join(suffix),
    };
    std::fs::canonicalize(&raw)
}

fn home_from_env() -> Result<PathBuf, io::Error> {
    match env::var("HOME") {
        Ok(value) => Ok(PathBuf::from(value)),
        Err(error) => {
            Err(io::Error::new(io::ErrorKind::NotFound, format!("HOME is not set: {error}")))
        }
    }
}

fn link_if_needed(target: &std::path::Path, link: &std::path::Path) -> Result<(), io::Error> {
    use std::fs;

    let already_correct = fs::read_link(link).is_ok_and(|existing| existing == target);
    if already_correct {
        return Ok(());
    }

    if link.exists() && !link.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} exists as a real directory; refusing to overwrite", link.display()),
        ));
    }

    match fs::remove_file(link) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(not(unix))]
    {
        let _ = target;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "symlinks are not supported on this platform",
        ))
    }
}

fn clear_scope_outputs(target_root: &std::path::Path, scope: GateScope) -> Result<(), io::Error> {
    let out_dir = target_root.join(".titania").join("out").join(scope.name());
    scope.lanes().iter().try_for_each(|lane| remove_lane_artifact(&out_dir, *lane))
}

fn remove_lane_artifact(out_dir: &std::path::Path, lane: Lane) -> Result<(), io::Error> {
    match std::fs::remove_file(out_dir.join(lane.file_stem()).with_extension("json")) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn aggregate_with_opts(
    scope: GateScope,
    emit: args::EmitFormat,
    out: Option<&std::path::PathBuf>,
) -> CliDisposition {
    env::current_dir().map_or_else(
        |error| current_dir_error(&error),
        |target_root| aggregate_from_root(&target_root, scope, emit, out),
    )
}

fn current_dir_error(error: &io::Error) -> CliDisposition {
    CliDisposition::internal_error(format!("InternalError: current directory unavailable: {error}"))
}

fn aggregate_after_moon(
    target_root: &std::path::Path,
    scope: GateScope,
    emit: args::EmitFormat,
    out: Option<&PathBuf>,
    hermetic: &moon::HermeticEnv,
) -> CliDisposition {
    if !hermetic.is_complete() && policy_scan_has_env_bypass(target_root, scope) {
        emit_hermetic_hint();
    }
    aggregate_from_root(target_root, scope, emit, out)
}

fn emit_hermetic_hint() {
    let hint = "hint: run `titania-check setup-hermetic` to create hermetic CARGO_HOME/RUSTUP_HOME for PolicyScan";
    drop(write_stderr_line(hint));
}

fn policy_scan_has_env_bypass(target_root: &std::path::Path, scope: GateScope) -> bool {
    let artifact =
        target_root.join(".titania").join("out").join(scope.name()).join("policy-scan.json");
    let Ok(content) = std::fs::read_to_string(&artifact) else {
        return false;
    };
    content.contains("BYPASS_ENV_CARGO_HOME") || content.contains("BYPASS_ENV_RUSTUP_HOME")
}
fn aggregate_from_root(
    target_root: &std::path::Path,
    scope: GateScope,
    emit: args::EmitFormat,
    out: Option<&std::path::PathBuf>,
) -> CliDisposition {
    match aggregate::report_json(target_root, scope) {
        Ok(report) => report_with_opts(&report, emit, out),
        Err(error) => CliDisposition::input_error(error.diagnostic()),
    }
}
fn report_with_opts(
    report: &aggregate::ReportJson,
    emit: args::EmitFormat,
    out: Option<&std::path::PathBuf>,
) -> CliDisposition {
    let stdout = match emit {
        args::EmitFormat::Human => report.render_human(),
        args::EmitFormat::Json => report.json().to_owned(),
    };

    if let Some(path) = out {
        write_report(path, &stdout, report.status())
    } else {
        CliDisposition::report(stdout, report_code(report.status()))
    }
}

fn write_report(
    path: &std::path::PathBuf,
    content: &str,
    status: aggregate::ReportStatus,
) -> CliDisposition {
    if let Err(e) = std::fs::write(path, content) {
        return CliDisposition::internal_error(format!(
            "InternalError: cannot write report to {}: {e}",
            path.display()
        ));
    }
    CliDisposition::silent(report_code(status))
}

const fn report_code(status: aggregate::ReportStatus) -> u8 {
    match status {
        aggregate::ReportStatus::Pass => EXIT_PASS,
        aggregate::ReportStatus::Reject => EXIT_REJECT,
        aggregate::ReportStatus::PolicyError => EXIT_POLICY_ERROR,
        aggregate::ReportStatus::InputError => EXIT_INPUT_ERROR,
    }
}

fn run_lane(lane: Lane) -> CliDisposition {
    let execution = titania_lanes::run_lane::execute_lane(lane);
    CliDisposition::lane_execution(map_lane_exit(execution.exit()), execution.stderr())
}

const fn map_lane_exit(exit: LaneExit) -> u8 {
    match exit {
        LaneExit::Clean | LaneExit::NotApplicable => EXIT_PASS,
        LaneExit::Violations => EXIT_REJECT,
        LaneExit::Usage => EXIT_INPUT_ERROR,
        LaneExit::Failure => EXIT_INTERNAL_ERROR,
    }
}

fn doctor_scope(scope: GateScope, emit: args::EmitFormat) -> CliDisposition {
    match doctor::render(scope, emit) {
        Ok(disposition) => disposition,
        Err(error) => CliDisposition::internal_error(format!("InternalError: {error}")),
    }
}

fn explain_rule(rule_id: &str) -> CliDisposition {
    explain::render(rule_id).map_or_else(
        |error| CliDisposition::input_error(error.to_string()),
        |output| CliDisposition::report(output, EXIT_PASS),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliDisposition {
    code: u8,
    stdout: Option<String>,
    diagnostic: Option<String>,
}

impl CliDisposition {
    const fn input_error(message: String) -> Self {
        Self { code: EXIT_INPUT_ERROR, stdout: None, diagnostic: Some(message) }
    }

    const fn internal_error(message: String) -> Self {
        Self { code: EXIT_INTERNAL_ERROR, stdout: None, diagnostic: Some(message) }
    }

    const fn report(stdout: String, code: u8) -> Self {
        Self { code, stdout: Some(stdout), diagnostic: None }
    }
    const fn silent(code: u8) -> Self {
        Self { code, stdout: None, diagnostic: None }
    }

    fn lane_execution(code: u8, stderr: &str) -> Self {
        let diagnostic = (!stderr.is_empty()).then(|| stderr.trim_end_matches('\n').to_owned());
        Self { code, stdout: None, diagnostic }
    }

    const fn code(&self) -> u8 {
        self.code
    }

    fn stdout(&self) -> Option<&str> {
        self.stdout.as_deref()
    }

    fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

fn exit(disposition: &CliDisposition) -> ExitCode {
    write_disposition(disposition).map_or_else(
        |_| ExitCode::from(EXIT_INTERNAL_ERROR),
        |()| ExitCode::from(disposition.code()),
    )
}

fn write_disposition(disposition: &CliDisposition) -> io::Result<()> {
    if let Some(stdout) = disposition.stdout() {
        stdout_is_silent(stdout)?;
    }
    if let Some(diagnostic) = disposition.diagnostic() {
        return write_stderr_line(diagnostic);
    }
    Ok(())
}

fn stdout_is_silent(text: &str) -> io::Result<()> {
    match write_stdout_line(text) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

fn write_stdout_line(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.write_all(b"\n")
}

fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}
