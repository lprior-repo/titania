use std::path::PathBuf;

use titania_lanes::CommandIn;

fn run_local_lane(target: &TargetProject, lane: LocalLane) -> LaneExit {
    let binary = match sibling_binary(lane.binary_name()) {
        Ok(binary) => binary,
        Err(error) => return failure_after_stderr(format_args!("[gauntlet] Failure: {error}")),
    };
    binary_status(target, &binary)
}

/// Resolves a gauntlet helper binary next to the current executable.
///
/// # Errors
///
/// Returns an error when the current executable cannot be resolved, has no parent
/// directory, or the named sibling binary is absent.
fn sibling_binary(binary_name: &str) -> Result<PathBuf, GauntletError> {
    let current = std::env::current_exe().map_err(|error| GauntletError::from(error.to_string()))?;
    let Some(dir) = current.parent() else {
        return Err(GauntletError::from("cannot resolve current Titania lane binary directory"));
    };
    let binary = dir.join(binary_name);
    if binary.is_file() { Ok(binary) } else { Err(GauntletError::from(missing_binary(binary_name, dir))) }
}

fn missing_binary(binary_name: &str, dir: &std::path::Path) -> String {
    let shown = dir.display();
    format!(
        "missing Titania lane binary `{binary_name}` beside `{shown}`; build/install titania-lanes lane binaries before running applicable target projects"
    )
}

fn binary_status(target: &TargetProject, binary: &std::path::Path) -> LaneExit {
    let Some(program) = binary.to_str() else {
        return failure_after_stderr(format_args!(
            "[gauntlet] Failure: Titania lane binary path is not valid UTF-8"
        ));
    };
    let mut cmd = match CommandIn::new(target, program) {
        Ok(command) => command,
        Err(error) => return failure_after_stderr(format_args!(
            "[gauntlet] Failure: cannot prepare Titania lane binary: {error}"
        )),
    };
    command_status(&mut cmd)
}

fn run_clippy_vb_compile(target: &TargetProject) -> LaneExit {
    cargo_status(
        target,
        &["clippy", "-p", "vb_compile", "--lib", "--", "-D", "warnings", "-A", "unsafe_code"],
    )
}

fn run_test(target: &TargetProject, group: &str) -> LaneExit {
    let args = vec!["test", "-p", "vb_compile", "--lib", group, "--", "--nocapture"];
    cargo_status(target, &args)
}

fn run_kani(target: &TargetProject, harness: &str) -> LaneExit {
    let args = vec!["kani", "--package", "vb_compile", "--harness", harness, "--quiet"];
    cargo_status(target, &args)
}

fn run_kani_default_unwind(target: &TargetProject, harness: &str) -> LaneExit {
    let args = vec![
        "kani",
        "--package",
        "vb_runtime",
        "--harness",
        harness,
        "--default-unwind",
        "1",
        "--quiet",
    ];
    cargo_status(target, &args)
}

/// Captures a cargo invocation in the target project.
///
/// # Errors
///
/// Returns an error when the cargo command cannot be prepared, fails to execute,
/// or its captured output cannot be represented by `CommandOutput`.
fn cargo_capture(
    target: &TargetProject,
    args: &[&str],
) -> Result<titania_lanes::CommandOutput, GauntletError> {
    let mut cmd = CommandIn::new(target, "cargo").map_err(|error| GauntletError::from(error.to_string()))?;
    let _ = cmd.inherit_env();
    let _ = cmd.env_remove("RUSTC_WRAPPER");
    let _ = cmd.env("SCCACHE_DISABLE", "1");
    let _ = cmd.args(args);
    cmd.run_capture().map_err(|error| GauntletError::from(error.to_string()))
}

fn cargo_status(target: &TargetProject, args: &[&str]) -> LaneExit {
    let Ok(mut cmd) = CommandIn::new(target, "cargo") else {
        return LaneExit::Violations;
    };
    let _ = cmd.args(args);
    command_status(&mut cmd)
}

fn command_status(cmd: &mut CommandIn<'_>) -> LaneExit {
    let _ = cmd.inherit_env();
    let _ = cmd.env_remove("RUSTC_WRAPPER");
    let _ = cmd.env("SCCACHE_DISABLE", "1");
    match cmd.run_status_raw() {
        Ok(status) if status.success() => LaneExit::Clean,
        Ok(_) => LaneExit::Violations,
        Err(error) => failure_after_stderr(format_args!(
            "[gauntlet] Failure: command execution failed: {error}"
        )),
    }
}
