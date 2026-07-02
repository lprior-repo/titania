fn run_fast_steps(target: &TargetProject, report: &mut LaneReport) -> LaneExit {
    let clippy = step(report, "STATIC-LINT-001", || run_clippy_vb_compile(target));
    let ignored = step(report, "ignored-fallible gate", || {
        run_local_lane(target, LocalLane::IgnoredFallible)
    });
    merge(merge(clippy, ignored), test_steps(target, report))
}

fn run_standard_steps(mode: Mode, target: &TargetProject, report: &mut LaneReport) -> LaneExit {
    if matches!(mode, Mode::Standard | Mode::Deep | Mode::Proof) {
        kani_steps(target, report, &STANDARD_KANI, run_kani)
    } else {
        LaneExit::Clean
    }
}

fn run_deep_steps(mode: Mode, target: &TargetProject, report: &mut LaneReport) -> LaneExit {
    if matches!(mode, Mode::Deep | Mode::Proof) {
        kani_steps(target, report, &DEEP_KANI, run_kani)
    } else {
        LaneExit::Clean
    }
}

fn run_proof_steps(
    mode: Mode,
    target: &TargetProject,
    packages: TargetPackages,
    report: &mut LaneReport,
) -> LaneExit {
    if mode != Mode::Proof {
        return LaneExit::Clean;
    }
    let drift =
        step(report, "DRIFT-STEPSTATE-001", || run_local_lane(target, LocalLane::StepstateMatrix));
    let admission = if packages.vb_runtime.is_present() {
        kani_steps(target, report, &ADMISSION_KANI, run_kani_default_unwind)
    } else {
        clean_after_stderr(format_args!(
            "[gauntlet] NotApplicable: package vb_runtime absent; skipping admission Kani checks"
        ))
    };
    if matches!(admission, LaneExit::Failure) {
        return LaneExit::Failure;
    }
    if write_stderr_line(format_args!(
        "[gauntlet] NOTE: Verus proofs (VERUS-EXPR-STACK-001, VERUS-SLOT-MAX-001) are WAIVED -- toolchain not installed"
    ))
    .is_err()
    {
        return LaneExit::Failure;
    }
    merge(drift, admission)
}

const fn label(m: Mode) -> &'static str {
    match m {
        Mode::Fast => "fast",
        Mode::Standard => "standard",
        Mode::Deep => "deep",
        Mode::Proof => "proof",
    }
}

fn step<F: FnOnce() -> LaneExit>(report: &mut LaneReport, label: &str, f: F) -> LaneExit {
    match f() {
        LaneExit::Clean | LaneExit::NotApplicable => pass_step_after_stderr(report, label),
        LaneExit::Violations => {
            lane_after_stderr(format_args!("[FAIL] {label}"), LaneExit::Violations)
        }
        LaneExit::Usage | LaneExit::Failure => {
            failure_after_stderr(format_args!("[ERROR] {label}"))
        }
    }
}

const fn merge(left: LaneExit, right: LaneExit) -> LaneExit {
    match (left, right) {
        (LaneExit::Failure | LaneExit::Usage, _) | (_, LaneExit::Failure | LaneExit::Usage) => {
            LaneExit::Failure
        }
        (LaneExit::Violations, _) | (_, LaneExit::Violations) => LaneExit::Violations,
        (LaneExit::Clean | LaneExit::NotApplicable, LaneExit::Clean)
        | (LaneExit::Clean, LaneExit::NotApplicable) => LaneExit::Clean,
        (LaneExit::NotApplicable, LaneExit::NotApplicable) => LaneExit::NotApplicable,
    }
}

fn test_steps(target: &TargetProject, report: &mut LaneReport) -> LaneExit {
    TEST_GROUPS.iter().fold(LaneExit::Clean, |exit_code, (name, group)| {
        merge(exit_code, step(report, name, || run_test(target, group)))
    })
}

fn kani_steps(
    target: &TargetProject,
    report: &mut LaneReport,
    steps: &[(&str, &str)],
    runner: fn(&TargetProject, &str) -> LaneExit,
) -> LaneExit {
    steps.iter().fold(LaneExit::Clean, |exit_code, (name, harness)| {
        merge(exit_code, step(report, name, || runner(target, harness)))
    })
}

fn exit_after_stderr(args: std::fmt::Arguments<'_>, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

fn pass_step_after_stderr(report: &mut LaneReport, label: &str) -> LaneExit {
    if write_stderr_line(format_args!("[PASS] {label}")).is_err() {
        return LaneExit::Failure;
    }
    report.record_pass();
    LaneExit::Clean
}

fn clean_after_stderr(args: std::fmt::Arguments<'_>) -> LaneExit {
    lane_after_stderr(args, LaneExit::Clean)
}

fn not_applicable_after_stderr(args: std::fmt::Arguments<'_>) -> LaneExit {
    lane_after_stderr(args, LaneExit::NotApplicable)
}

fn failure_after_stderr(args: std::fmt::Arguments<'_>) -> LaneExit {
    lane_after_stderr(args, LaneExit::Failure)
}

fn lane_after_stderr(args: std::fmt::Arguments<'_>, code: LaneExit) -> LaneExit {
    if write_stderr_line(args).is_err() {
        return LaneExit::Failure;
    }
    code
}

/// Writes a formatted line to stderr.
///
/// # Errors
///
/// Returns the underlying I/O error when locking or writing to stderr fails.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
