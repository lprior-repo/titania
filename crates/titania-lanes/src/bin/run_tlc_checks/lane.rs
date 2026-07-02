#![expect(
    clippy::redundant_pub_crate,
    reason = "lane entrypoint is called by the private run_tlc_checks wrapper module"
)]

use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneExit, current_target_project, exit};

/// TLC jar path pinned by `mise` in the bash original.
const TLC_JAR: &str = "/home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar";
const TLA_DIR: &str = "verification/tla";
const SEED: &str = "0";

#[derive(Default, Clone, Copy)]
struct RunSummary {
    had_runs: bool,
    any_failed: bool,
}

impl RunSummary {
    const fn record(mut self, passed: bool) -> Self {
        self.had_runs = true;
        self.any_failed = self.any_failed || !passed;
        self
    }
}

pub(super) fn main_exit() -> ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(err) => {
            return exit_after_stderr_line(
                format_args!("[run-tlc-checks] cannot resolve target project: {err}"),
                LaneExit::Usage,
            );
        }
    };
    run_for_target(&target)
}

fn run_for_target(target: &TargetProject) -> ExitCode {
    let tla_dir = target.as_std_path().join(TLA_DIR);
    if !tla_dir.is_dir() {
        return no_tla_dir_exit(&tla_dir);
    }
    let cfg_files = collect_cfg_files(&tla_dir);
    if cfg_files.is_empty() {
        return no_cfg_exit(&tla_dir);
    }
    summarize_exit(run_cfg_pairs(target, &cfg_files))
}

fn no_tla_dir_exit(tla_dir: &Path) -> ExitCode {
    exit_after_stderr_line(
        format_args!(
            "[run-tlc-checks] no verification/tla directory found at {}; skipped",
            tla_dir.display()
        ),
        LaneExit::Clean,
    )
}

fn no_cfg_exit(tla_dir: &Path) -> ExitCode {
    exit_after_stderr_line(
        format_args!("[run-tlc-checks] no .cfg files found in {}; skipped", tla_dir.display()),
        LaneExit::Clean,
    )
}

fn summarize_exit(summary: RunSummary) -> ExitCode {
    if !summary.had_runs {
        return exit_after_stderr_line(
            format_args!("[run-tlc-checks] no .tla/.cfg pairs found; nothing to check"),
            LaneExit::Clean,
        );
    }
    if summary.any_failed { exit(LaneExit::Violations) } else { exit(LaneExit::Clean) }
}

fn collect_cfg_files(tla_dir: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(tla_dir) {
        Ok(entries) => entries,
        Err(err) => {
            return read_dir_error(tla_dir, &err);
        }
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|path| is_cfg_file(path.as_path()))
        .collect()
}

fn read_dir_error(tla_dir: &Path, err: &io::Error) -> Vec<PathBuf> {
    match write_stderr_line(format_args!(
        "[run-tlc-checks] cannot read {}: {err}",
        tla_dir.display()
    )) {
        Ok(()) | Err(_) => Vec::new(),
    }
}

fn is_cfg_file(path: &Path) -> bool {
    path.is_file() && path.extension() == Some(OsStr::new("cfg"))
}

fn run_cfg_pairs(target: &TargetProject, cfg_files: &[PathBuf]) -> RunSummary {
    cfg_files
        .iter()
        .filter_map(|cfg| checked_tla_pair(cfg))
        .fold(RunSummary::default(), |summary, (cfg, tla)| run_pair(target, summary, &cfg, &tla))
}

fn checked_tla_pair(cfg: &Path) -> Option<(PathBuf, PathBuf)> {
    let tla = cfg.with_extension("tla");
    if tla.is_file() { Some((cfg.to_path_buf(), tla)) } else { None }
}

fn run_pair(target: &TargetProject, summary: RunSummary, cfg: &Path, tla: &Path) -> RunSummary {
    let wrote_status = write_stderr_line(format_args!("Checking {}...", tla.display())).is_ok();
    let passed = run_tlc(target, cfg, tla);
    summary.record(wrote_status && passed)
}

fn run_tlc(target: &TargetProject, cfg: &Path, tla: &Path) -> bool {
    let cfg_arg = cfg.display().to_string();
    let tla_arg = tla.display().to_string();
    let mut command = match CommandIn::new(target, "java") {
        Ok(command) => command,
        Err(err) => {
            return command_error_false(format_args!(
                "[run-tlc-checks] failed to prepare java: {err}"
            ));
        }
    };
    append_tlc_args(&mut command, &cfg_arg, &tla_arg);
    execute_tlc(&command)
}

fn append_tlc_args<'a>(command: &mut CommandIn<'a>, cfg_arg: &'a str, tla_arg: &'a str) {
    let _ = command.inherit_env();
    let _ = command.arg("-cp").arg(TLC_JAR).arg("tlc2.TLC").arg("-seed");
    let _ = command.arg(SEED).arg("-config").arg(cfg_arg).arg(tla_arg);
}

fn execute_tlc(command: &CommandIn<'_>) -> bool {
    match command.run_capture_raw() {
        Ok(out) => print_tlc_tail(out.stdout(), out.stderr()).is_ok() && out.status().success(),
        Err(err) => {
            command_error_false(format_args!("[run-tlc-checks] failed to spawn java: {err}"))
        }
    }
}

/// # Errors
///
/// Returns an error when stdout cannot be written.
fn print_tlc_tail(stdout: &[u8], stderr: &[u8]) -> io::Result<()> {
    let mut combined = String::from_utf8_lossy(stdout).into_owned();
    combined.push('\n');
    combined.push_str(&String::from_utf8_lossy(stderr));
    tail_lines(&combined, 3).try_for_each(|line| write_stdout_line(format_args!("{line}")))
}

fn tail_lines(text: &str, n: usize) -> impl Iterator<Item = &str> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines.into_iter().skip(start)
}

fn command_error_false(args: std::fmt::Arguments<'_>) -> bool {
    match write_stderr_line(args) {
        Ok(()) | Err(_) => false,
    }
}

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// # Errors
///
/// Returns an error when stderr cannot be written.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

/// # Errors
///
/// Returns an error when stdout cannot be written.
fn write_stdout_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_fmt(args)?;
    stdout.write_all(b"\n")
}
