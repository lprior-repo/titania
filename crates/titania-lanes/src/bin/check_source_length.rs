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

use titania_lanes::{LaneExit, LaneReport, current_target_project, exit};

#[path = "check_source_length/compile_split.rs"]
mod compile_split;
#[path = "check_source_length/function_scan.rs"]
mod function_scan;
#[path = "check_source_length/ledger.rs"]
mod ledger;
#[path = "check_source_length/mutants.rs"]
mod mutants;
#[path = "check_source_length/paths.rs"]
mod paths;
#[path = "check_source_length/source_limit.rs"]
mod source_limit;

const FN_LINE_LIMIT: usize = 25;
const SOURCE_LINE_LIMIT: usize = 300;
const LEDGER_PATH: &str = ".config/source-length-exceptions.txt";

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            eprintln!("[check-source-length] cannot resolve target project: {error}");
            return exit(LaneExit::Usage);
        }
    };
    let mut report = LaneReport::new();
    run(target.as_std_path(), &mut report);
    print_and_exit(&report)
}

fn run(root: &Path, report: &mut LaneReport) {
    mutants::check_mutants_residue(root, report);
    compile_split::check_compile_split_sources(root, report);
    let tracked = paths::tracked_rust_files(root);
    let exceptions = ledger::load_ledger(root, report);
    if let Some(files) = tracked.as_deref() {
        source_limit::check_source_line_limit(root, files, &exceptions, report);
        check_hot_functions(root, files, report);
    }
}

fn check_hot_functions(root: &Path, files: &[std::path::PathBuf], report: &mut LaneReport) {
    for file in files.iter().filter(|file| paths::is_titania_hot_source(root, file)) {
        function_scan::check_file(root, file, report);
    }
}

fn print_and_exit(report: &LaneReport) -> std::process::ExitCode {
    let rendered = report.render();
    if !rendered.is_empty() {
        eprint!("{rendered}");
    }
    if report.is_clean() { exit(LaneExit::Clean) } else { exit(LaneExit::Violations) }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_methods,
        clippy::disallowed_macros,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::arithmetic_side_effects,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::panic_in_result_fn,
        clippy::cognitive_complexity,
        clippy::doc_markdown,
        clippy::excessive_nesting,
        clippy::many_single_char_names,
        clippy::integer_division,
        clippy::integer_division_remainder_used,
        clippy::needless_borrow,
        clippy::needless_pass_by_value,
        clippy::format_collect,
        reason = "Tests are exempt from the strict production deny list per project doctrine."
    )]
    use super::*;

    /// Writes a single fixture file under `root`/`rel`, creating parent
    /// directories as needed.
    ///
    /// # Errors
    /// Returns the underlying I/O error from `create_dir_all` or `write`.
    fn fixture_file(root: &Path, rel: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
        Ok(())
    }

    fn long_source(lines: usize) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for idx in 0..lines {
            writeln!(out, "pub const L{idx}: usize = {idx};").expect("writing to String is infallible");
        }
        out
    }

    fn long_function() -> String {
        use std::fmt::Write as _;
        let mut body = String::new();
        for idx in 0_usize..26_usize {
            writeln!(body, "    let _v{idx} = {idx};").expect("writing to String is infallible");
        }
        format!("fn oversized() {{\n{body}}}\n")
    }

    #[test]
    fn missing_source_length_ledger_keeps_line_limit_active()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        fixture_file(
            temp.path(),
            "crates/titania-lanes/src/lib.rs",
            &long_source(SOURCE_LINE_LIMIT + 1),
        )?;

        let mut report = LaneReport::new();
        run(temp.path(), &mut report);

        assert!(report.findings().iter().any(|finding| {
            finding.rule() == "SRC-LINE-LIMIT"
                && finding.path() == "crates/titania-lanes/src/lib.rs"
        }));
        Ok(())
    }

    #[test]
    fn src_bin_production_functions_are_scanned() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        fixture_file(temp.path(), "crates/titania-lanes/src/bin/oversized.rs", &long_function())?;

        let mut report = LaneReport::new();
        run(temp.path(), &mut report);

        assert!(report.findings().iter().any(|finding| {
            finding.rule() == "FN-LINE-LIMIT"
                && finding.path() == "crates/titania-lanes/src/bin/oversized.rs"
        }));
        Ok(())
    }
}
