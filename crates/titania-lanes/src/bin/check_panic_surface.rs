//! Scans `crates/*/src` for production panic/assert macros.
//!
//! Rust re-implementation of the bash lane `scripts/check-panic-surface.sh`. Run via
//! `cargo run --bin check-panic-surface --` from the repository root or
//! via the matching Moon task in `.moon/tasks/all.yml`.
//!
//! ## Behavior parity
//! Mirrors the bash's exclusion globs and per-line allowlist rules:
//!
//! 1. **Path exclusions** — skip tests, benches, examples, fuzz harnesses,
//!    `target/`, `.beads/`, fixtures, `build.rs`, `*_tests.rs`, `tests.rs`,
//!    `lifecycle_tests/`, `kani*.rs`, `models/loom/**`, `proofs/**`, etc.
//! 2. **Production path filter** — only lines outside `#[cfg(test)]`,
//!    `#[cfg(kani)]`, and `#[kani::proof]` blocks count.
//! 3. **Comment skip** — lines whose payload (after the `<file:line>` prefix)
//!    starts with `//` are not violations (matches `rg` post-filter).
//! 4. **Pattern** — `(^|[^A-Za-z0-9_])(assert!|assert_eq!|assert_ne!|unreachable!)`
//!
//! Each violation becomes a typed `Finding`; the report's `render()`
//! gives a stable `path:line: rule -- message` line. The bash's
//! `ViolationFound` / `NoViolationFound` summaries are preserved.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_panic_surface/output.rs"]
/// Stderr and exit-code helpers for the panic-surface lane.
pub mod output;
#[path = "check_panic_surface/paths.rs"]
/// Source file collection and path exclusion helpers.
pub mod paths;
#[path = "check_panic_surface/scan.rs"]
/// Panic macro scanner state machine.
pub mod scan;

use std::path::Path;

use titania_lanes::{LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit};

/// Macros the bash lane flags. Kept as a single array so additions land
/// in one obvious place.
const PANIC_MACROS: &[&str] = &["assert!", "assert_eq!", "assert_ne!", "unreachable!"];
const PANIC_SURFACE_RULE: &str = "PANIC_SURFACE_001";

/// Path segments whose presence means the file is non-production.
const EXCLUDED_SEGMENTS: &[&str] = &[
    "/workspace_tests/",
    "/test_loop_inventory/",
    "/tests/",
    "/lifecycle_tests/",
    "/benches/",
    "/examples/",
    "/proofs/",
    "/models/loom/",
    "/target/",
    "/.beads/",
    "/fixtures/",
    "/fuzz/",
    "/titania-lanes/src/bin/",
];

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(error) => {
            return output::exit_after_stderr_line(
                &format!("[check-panic-surface] target discovery failed: {error}"),
                LaneExit::Failure,
            );
        }
    };
    if output::write_scan_header().is_err() {
        return exit(LaneExit::Failure);
    }
    let rule = match panic_surface_rule() {
        Ok(rule) => rule,
        Err(error) => {
            return output::exit_after_stderr_line(
                &format!("[check-panic-surface] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    emit_result(&scan_target(target.as_std_path(), &rule))
}

/// Build the configured panic-surface rule identifier.
///
/// # Errors
///
/// Returns the invalid rule-id error if the static rule id violates the shared grammar.
fn panic_surface_rule() -> Result<RuleId, RuleIdError> {
    RuleId::new(PANIC_SURFACE_RULE)
}

fn scan_target(root: &Path, rule: &RuleId) -> LaneReport {
    let mut report = LaneReport::new();
    for file in paths::collect_source_files(root) {
        scan::scan_file(root, &file, rule, &mut report);
    }
    report
}

fn emit_result(report: &LaneReport) -> std::process::ExitCode {
    if output::write_stderr(&report.render()).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() {
        output::exit_after_stderr_line("NoViolationFound", LaneExit::Clean)
    } else {
        output::exit_after_stderr_line(
            "ViolationFound: production panic/assert macro surface is non-empty",
            LaneExit::Violations,
        )
    }
}
