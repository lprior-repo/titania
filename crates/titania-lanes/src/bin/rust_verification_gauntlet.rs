//! Dispatcher for fast | standard | deep | proof Rust verification lanes.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::get_unwrap,
    clippy::arithmetic_side_effects,
    clippy::dbg_macro,
    clippy::as_conversions
)]
#![forbid(unsafe_code)]
use std::{
    env,
    io::{self, Write},
};

use serde_json::Value;
use titania_core::TargetProject;
use titania_lanes::{LaneExit, LaneReport, current_target_project, exit};

#[derive(Debug)]
struct GauntletError(String);

impl std::fmt::Display for GauntletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GauntletError {}

impl From<String> for GauntletError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&'static str> for GauntletError {
    fn from(value: &'static str) -> Self {
        Self(value.to_owned())
    }
}

include!("rust_verification_gauntlet/commands.rs");

const TEST_GROUPS: [(&str, &str); 4] = [
    ("UNIT-EXPR-BYTESTACK-001", "expression_bytecode"),
    ("UNIT-SLOT-COMPILER-001", "slot_compiler"),
    ("UNIT-LOWER-DO-001", "lower"),
    ("POST-009-VALIDATE-001", "lower_steps"),
];
const STANDARD_KANI: [(&str, &str); 9] = [
    ("KANI-EXPR-BYTECODE-001", "compile_expr_to_bytecode_overflow"),
    ("KANI-SLOT-REF-001", "lower_slot_reference_valid"),
    ("KANI-SLOT-REF-001b", "lower_slot_reference_with_path_creates_accessor"),
    ("KANI-CONSTANT-POOL-001", "push_constant_overflow"),
    ("KANI-CONSTANT-POOL-001b", "push_constant_isolation"),
    ("KANI-CONSTANT-POOL-001c", "slot_count_overflow_at_max"),
    ("KANI-ACCESSOR-REF-001", "lower_accessor_reference_numeric"),
    ("KANI-ACCESSOR-REF-001b", "accessor_index_assignment"),
    ("KANI-ACCESSOR-REF-001c", "rejects_non_numeric_accessor_path"),
];
const DEEP_KANI: [(&str, &str); 2] = [
    ("INV-007-NODEDUP-001", "node_id_uniqueness"),
    ("INV-007-NODEDUP-001b", "step_idx_ordering_preserved"),
];
const ADMISSION_KANI: [(&str, &str); 3] = [
    ("KANI-ADMISSION-001-MALFORMED", "strict_admission_invalid_artifact_cases_reject"),
    ("KANI-ADMISSION-001-CAPABILITY", "strict_admission_invalid_capability_rejects"),
    ("KANI-ADMISSION-001-VALID", "strict_admission_valid_artifact_admits"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Fast,
    Standard,
    Deep,
    Proof,
}

impl Mode {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "fast" => Some(Self::Fast),
            "standard" => Some(Self::Standard),
            "deep" => Some(Self::Deep),
            "proof" | "all" => Some(Self::Proof),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackagePresence {
    Present,
    Absent,
}

impl PackagePresence {
    const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TargetPackages {
    vb_compile: PackagePresence,
    vb_runtime: PackagePresence,
}

impl TargetPackages {
    /// Discovers whether the target project contains packages needed by the gauntlet.
    ///
    /// # Errors
    ///
    /// Returns an error when cargo metadata cannot be captured, decoded as UTF-8,
    /// or parsed as JSON.
    fn discover(target: &TargetProject) -> Result<Self, GauntletError> {
        let output = cargo_capture(target, &["metadata", "--format-version", "1", "--no-deps"])?;
        let text = output.stdout_str().map_err(|error| error.to_string())?;
        let metadata = serde_json::from_str::<Value>(text).map_err(|error| error.to_string())?;
        Ok(Self {
            vb_compile: package_presence(&metadata, "vb_compile"),
            vb_runtime: package_presence(&metadata, "vb_runtime"),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalLane {
    IgnoredFallible,
    StepstateMatrix,
}

impl LocalLane {
    const fn binary_name(self) -> &'static str {
        match self {
            Self::IgnoredFallible => "check-ignored-fallible-results",
            Self::StepstateMatrix => "check-stepstate-matrix",
        }
    }
}

fn package_presence(metadata: &Value, name: &str) -> PackagePresence {
    let present = metadata.get("packages").and_then(Value::as_array).is_some_and(|packages| {
        packages.iter().any(|package| package.get("name").and_then(Value::as_str) == Some(name))
    });
    if present { PackagePresence::Present } else { PackagePresence::Absent }
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode_str = args.first().map_or("fast", String::as_str);
    let Some(mode) = Mode::parse(mode_str) else {
        return exit_after_stderr(
            format_args!("usage: rust-verification-gauntlet <fast|standard|deep|proof|all>"),
            LaneExit::Usage,
        );
    };
    let target = match current_target_project() {
        Ok(target) => target,
        Err(err) => {
            return exit_after_stderr(
                format_args!("[gauntlet] cannot resolve target project: {err}"),
                LaneExit::Usage,
            );
        }
    };
    exit(run(mode, &target))
}

fn run(mode: Mode, target: &TargetProject) -> LaneExit {
    let packages = match TargetPackages::discover(target) {
        Ok(packages) => packages,
        Err(error) => {
            return failure_after_stderr(format_args!(
                "[gauntlet] cannot inspect target packages: {error}"
            ));
        }
    };
    let mut report = LaneReport::new();
    let result = dispatch(mode, target, packages, &mut report);
    if !report.is_clean() && write_stderr_line(format_args!("{}", report.render())).is_err() {
        return LaneExit::Failure;
    }
    result
}

fn dispatch(
    mode: Mode,
    target: &TargetProject,
    packages: TargetPackages,
    report: &mut LaneReport,
) -> LaneExit {
    if write_stderr_line(format_args!("[gauntlet] mode: {}", label(mode))).is_err() {
        return LaneExit::Failure;
    }
    if !packages.vb_compile.is_present() {
        return not_applicable_after_stderr(format_args!(
            "[gauntlet] NotApplicable: package vb_compile absent; skipping compile gauntlet"
        ));
    }
    let fast = run_fast_steps(target, report);
    let standard = run_standard_steps(mode, target, report);
    let deep = run_deep_steps(mode, target, report);
    merge(merge(fast, standard), merge(deep, run_proof_steps(mode, target, packages, report)))
}

include!("rust_verification_gauntlet/dispatch.rs");
