use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::ExitCode,
};

use titania_lanes::{
    Finding, LaneExit, LaneReport, RuleId, SourceLine, current_target_project, exit,
    helpers::{line_no_from_idx, relative_path},
};


/// Default forbidden tokens (Holzman Rust slice 1).
const DEFAULT_FORBIDDEN: &[&str] =
    &["panic!", "unwrap", "expect", "todo!", "unimplemented!", "dbg!"];
const FORBIDDEN_FLAG: &str = "--forbidden=";
const FORBIDDEN_RULE: &str = "FORBIDDEN_001";

fn main_exit(args: &[String]) -> ExitCode {
    let forbidden = parse_forbidden(args);
    if forbidden.is_empty() {
        return exit_after_io(
            write_stderr_line(format_args!("[forbidden-scan] no forbidden tokens configured")),
            LaneExit::Usage,
        );
    }
    let root = match target_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let rule = match RuleId::new(FORBIDDEN_RULE) {
        Ok(rule) => rule,
        Err(error) => {
            return exit_after_io(
                write_stderr_line(format_args!(
                    "[forbidden-scan] rule id configuration error: {error}"
                )),
                LaneExit::Failure,
            );
        }
    };
    if emit_scan_header(&root, &forbidden).is_err() {
        return exit(LaneExit::Failure);
    }
    scan_and_exit(&root, &forbidden, &rule)
}

/// Resolve the target-project root used as the scan base.
///
/// # Errors
///
/// Returns an exit code after emitting a usage message when the target
/// project cannot be resolved from the current environment.
fn target_root() -> Result<PathBuf, ExitCode> {
    current_target_project().map(|target| target.as_std_path().to_path_buf()).map_err(|error| {
        exit_after_io(
            write_stderr_line(format_args!(
                "[forbidden-scan] cannot resolve target project: {error}"
            )),
            LaneExit::Usage,
        )
    })
}

/// Emit the scanner header before findings.
///
/// # Errors
///
/// Returns the underlying stderr write error if a header line cannot be emitted.
fn emit_scan_header(root: &Path, forbidden: &[ForbiddenToken]) -> io::Result<()> {
    write_stderr_line(format_args!("CWD: {}", root.display()))?;
    write_stderr_line(format_args!("ScanDomain: crates/*/src"))?;
    write_stderr_line(format_args!(
        "ForbiddenTokens: {}",
        forbidden.iter().map(ForbiddenToken::as_str).collect::<Vec<_>>().join(",")
    ))
}

fn scan_and_exit(root: &Path, forbidden: &[ForbiddenToken], rule: &RuleId) -> ExitCode {
    let mut report = LaneReport::new();
    let files = collect_source_files(root);
    let findings: Vec<Finding> = files
        .iter()
        .inspect(|_| report.record_scan())
        .flat_map(|file| scan_file(root, file, forbidden, rule))
        .collect();
    report.extend_finding(findings);
    if write_stderr_raw(format_args!("{}", report.render())).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { clean_exit() } else { violations_exit() }
}

fn clean_exit() -> ExitCode {
    exit_after_io(write_stderr_line(format_args!("NoViolationFound")), LaneExit::Clean)
}

fn violations_exit() -> ExitCode {
    exit_after_io(
        write_stderr_line(format_args!("ViolationFound: forbidden token surface is non-empty")),
        LaneExit::Violations,
    )
}

fn exit_after_io(result: io::Result<()>, success: LaneExit) -> ExitCode {
    match result {
        Ok(()) => exit(success),
        Err(_error) => exit(LaneExit::Failure),
    }
}

fn parse_forbidden(args: &[String]) -> Vec<ForbiddenToken> {
    let override_set = args
        .iter()
        .find(|arg| arg.starts_with(FORBIDDEN_FLAG))
        .map(|arg| parse_override_set(arg.as_str()));
    match override_set {
        Some(set) if !set.is_empty() => set,
        Some(_) | None => default_forbidden_set(),
    }
}

fn parse_override_set(arg: &str) -> Vec<ForbiddenToken> {
    let body = arg.strip_prefix(FORBIDDEN_FLAG).map_or("", |body| body);
    body.split(',').filter_map(ForbiddenToken::parse).collect()
}

fn default_forbidden_set() -> Vec<ForbiddenToken> {
    DEFAULT_FORBIDDEN.iter().filter_map(|s| ForbiddenToken::parse(s)).collect()
}

fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let Ok(entries) = std::fs::read_dir(crates_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path().join("src"))
        .filter(|p| p.is_dir())
        .flat_map(walk_rust_files)
        .collect()
}

fn walk_rust_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir];
    while let Some(top) = stack.pop() {
        append_rust_files(&top, &mut stack, &mut out);
    }
    out.sort();
    out
}

fn append_rust_files(top: &Path, stack: &mut Vec<PathBuf>, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(top) else {
        return;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .for_each(|path| record_path(path, stack, out));
}

fn record_path(path: PathBuf, stack: &mut Vec<PathBuf>, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        stack.push(path);
    } else if path.extension().is_some_and(|e| e == "rs") {
        out.push(path);
    }
}

fn scan_file(
    root: &Path,
    path: &Path,
    forbidden: &[ForbiddenToken],
    rule: &RuleId,
) -> Vec<Finding> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let display = relative_path(root, path);
    scan_content(&content, &display, forbidden, rule)
}

fn scan_content(
    content: &str,
    display: &str,
    forbidden: &[ForbiddenToken],
    rule: &RuleId,
) -> Vec<Finding> {
    let mut block_comment = false;
    content
        .lines()
        .enumerate()
        .flat_map(|(idx, line)| {
            let source_line = SourceLine::parse(line, &mut block_comment);
            scan_source_line(&source_line, idx, display, forbidden, rule)
        })
        .collect()
}

fn scan_source_line(
    line: &SourceLine<'_>,
    idx: usize,
    display: &str,
    forbidden: &[ForbiddenToken],
    rule: &RuleId,
) -> Vec<Finding> {
    if line.is_non_code() {
        return Vec::new();
    }
    let line_no = line_no_from_idx(idx);
    forbidden
        .iter()
        .filter(|token| token.is_present_in(line.code()))
        .map(|token| {
            Finding::new(
                rule.clone(),
                display,
                line_no,
                format!("forbidden token `{}`", token.as_str()),
            )
        })
        .collect()
}

/// Write one formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

/// Write raw formatted text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_raw(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
