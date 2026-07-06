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

pub use titania_lanes::SourceLineState;

use titania_lanes::{
    CommandIn, Finding, LaneExit, LaneReport, RuleId, RuleIdError, SourceLine,
    current_target_project, exit,
};

/// Legacy summary rule retained in stderr compatibility text for old consumers.
const PANIC_SURFACE_RULE: &str = "PANIC_SURFACE_001";

/// Panic/assert macro to exact Holzman rule mapping.
#[derive(Clone, Copy)]
pub struct PanicMacroRule {
    macro_name: &'static str,
    rule_id: &'static str,
}

impl PanicMacroRule {
    /// Macro token matched in production Rust source.
    #[must_use]
    pub const fn macro_name(self) -> &'static str {
        self.macro_name
    }

    /// Exact v1 rule identifier for this macro.
    #[must_use]
    pub const fn rule_id(self) -> &'static str {
        self.rule_id
    }
}

/// Macros the panic scan lane flags. Kept as a single matrix so additions
/// land with their exact rule identifier.
const PANIC_MACROS: &[PanicMacroRule] = &[
    PanicMacroRule { macro_name: "assert!", rule_id: "HOLZMAN_PANIC_ASSERT" },
    PanicMacroRule { macro_name: "assert_eq!", rule_id: "HOLZMAN_PANIC_ASSERT_EQ" },
    PanicMacroRule { macro_name: "assert_ne!", rule_id: "HOLZMAN_PANIC_ASSERT_NE" },
    PanicMacroRule { macro_name: "unreachable!", rule_id: "HOLZMAN_PANIC_UNREACHABLE" },
];

/// Tool required by the v1 panic-scan lane contract.
const RG_TOOL: &str = "rg";

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
    if !rg_available(&target) {
        return output::exit_after_stderr_line(
            "InfraFailure: tool rg unavailable for panic-scan",
            LaneExit::Failure,
        );
    }
    if let Err(error) = panic_surface_rules() {
        return output::exit_after_stderr_line(
            &format!("[check-panic-surface] rule id configuration error: {error}"),
            LaneExit::Failure,
        );
    }
    emit_result(&scan_target(target.as_std_path()))
}

/// Validate the configured panic-surface rule identifiers.
///
/// # Errors
///
/// Returns the invalid rule-id error if any static rule id violates the shared grammar.
fn panic_surface_rules() -> Result<(), RuleIdError> {
    PANIC_MACROS.iter().try_for_each(|rule| RuleId::new(rule.rule_id()).map(|_| ()))
}

fn rg_available(target: &titania_core::TargetProject) -> bool {
    CommandIn::new(target, RG_TOOL)
        .and_then(|mut cmd| {
            let _ = cmd.arg("--version").inherit_env();
            cmd.run_capture_raw()
        })
        .is_ok_and(|output| output.success())
}

fn scan_target(root: &Path) -> LaneReport {
    let mut report = LaneReport::new();
    paths::collect_source_files(root)
        .into_iter()
        .fold((), |(), file| scan::scan_file(root, &file, &mut report));
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
