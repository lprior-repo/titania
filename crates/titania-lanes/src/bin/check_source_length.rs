//! Enforces per-function logical line cap + tracked source length limit with ledger.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/check-source-length.sh`. Run via
//! `cargo run --bin check-source-length --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::path::Path;

use titania_lanes::{LaneExit, LaneReport, RuleIdError, current_target_project, exit};

#[path = "check_source_length/compile_split.rs"]
/// Legacy compile-split layout checks.
pub mod compile_split;
#[path = "check_source_length/function_scan.rs"]
/// Logical function length scanner.
pub mod function_scan;
#[path = "check_source_length/ledger.rs"]
/// Source-length exception ledger parser.
pub mod ledger;
#[path = "check_source_length/mutants.rs"]
/// Cargo-mutants residue scanner.
pub mod mutants;
#[path = "check_source_length/paths.rs"]
/// Source path classification helpers.
pub mod paths;
#[path = "check_source_length/source_limit.rs"]
/// Physical source line limit checker.
pub mod source_limit;

const FN_LINE_LIMIT: usize = 25;
const SOURCE_LINE_LIMIT: usize = 300;
const LEDGER_PATH: &str = ".config/source-length-exceptions.txt";

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return exit_after_stderr_line(
                format_args!("[check-source-length] cannot resolve target project: {error}"),
                LaneExit::Usage,
            );
        }
    };
    let mut report = LaneReport::new();
    if let Err(error) = run(target.as_std_path(), &mut report) {
        return exit_after_stderr_line(
            format_args!("[check-source-length] rule id configuration error: {error}"),
            LaneExit::Failure,
        );
    }
    print_and_exit(&report)
}

/// Run all source-length checks.
///
/// # Errors
///
/// Returns a rule-id construction error from any source-length subcheck.
fn run(root: &Path, report: &mut LaneReport) -> Result<(), RuleIdError> {
    mutants::check_mutants_residue(root, report)?;
    compile_split::check_compile_split_sources(root, report)?;
    let tracked = paths::tracked_rust_files(root);
    let exceptions = ledger::load_ledger(root, report)?;
    if let Some(files) = tracked.as_deref() {
        source_limit::check_source_line_limit(root, files, &exceptions, report)?;
        check_hot_functions(root, files, report)?;
    }
    Ok(())
}

/// Check hot-source functions for logical line length.
///
/// # Errors
///
/// Returns a rule-id construction error from the function scanner.
fn check_hot_functions(
    root: &Path,
    files: &[std::path::PathBuf],
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    files
        .iter()
        .filter(|file| paths::is_titania_hot_source(root, file))
        .try_for_each(|file| function_scan::check_file(root, file, report))
}

fn print_and_exit(report: &LaneReport) -> std::process::ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() && write_stderr_raw(format_args!("{rendered}")).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

fn exit_after_stderr_line(
    args: std::fmt::Arguments<'_>,
    success: LaneExit,
) -> std::process::ExitCode {
    if write_stderr_line(args).is_err() {
        return exit(LaneExit::Failure);
    }
    exit(success)
}

/// Write one formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

/// Write raw formatted text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_raw(args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn fixture_file(root: &Path, rel: &str, text: &str) -> Result<(), String> {
        let path = root.join(rel);
        let Some(parent) = path.parent() else {
            return Err("test path has parent".to_string());
        };
        std::fs::create_dir_all(parent).map_err(|error| format!("create parent dirs: {error}"))?;
        std::fs::write(path, text).map_err(|error| format!("write test file: {error}"))
    }

    fn long_source(lines: usize) -> String {
        (0..lines).fold(String::new(), |mut out, idx| {
            let _write = writeln!(out, "pub const L{idx}: usize = {idx};");
            out
        })
    }

    fn long_function() -> String {
        let body = (0_usize..26_usize).fold(String::new(), |mut out, idx| {
            let _write = writeln!(out, "    let _v{idx} = {idx};");
            out
        });
        format!("fn oversized() {{\n{body}}}\n")
    }

    #[test]
    fn missing_source_length_ledger_keeps_line_limit_active() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|error| format!("tempdir: {error}"))?;
        fixture_file(
            temp.path(),
            "crates/titania-lanes/src/lib.rs",
            &long_source(SOURCE_LINE_LIMIT + 1),
        )?;

        let mut report = LaneReport::new();
        run(temp.path(), &mut report).map_err(|error| format!("run: {error}"))?;

        if report.findings().iter().any(|finding| {
            finding.rule().as_str() == "SRC_LINE_LIMIT"
                && finding.path() == "crates/titania-lanes/src/lib.rs"
        }) {
            Ok(())
        } else {
            Err("missing SRC_LINE_LIMIT finding".to_string())
        }
    }

    #[test]
    fn src_bin_production_functions_are_scanned() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|error| format!("tempdir: {error}"))?;
        fixture_file(temp.path(), "crates/titania-lanes/src/bin/oversized.rs", &long_function())?;

        let mut report = LaneReport::new();
        run(temp.path(), &mut report).map_err(|error| format!("run: {error}"))?;

        if report.findings().iter().any(|finding| {
            finding.rule().as_str() == "FN_LINE_LIMIT"
                && finding.path() == "crates/titania-lanes/src/bin/oversized.rs"
        }) {
            Ok(())
        } else {
            Err("missing FN_LINE_LIMIT finding".to_string())
        }
    }
}
