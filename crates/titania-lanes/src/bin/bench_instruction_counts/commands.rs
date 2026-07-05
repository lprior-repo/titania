/// `cargo bench --bench NAME --all-features --no-run` (via rustup).
///
/// # Errors
///
/// Returns an error when the compile command cannot be prepared, spawned, or exits unsuccessfully.
fn run_compile(
    target: &TargetProject,
    target_dir: &Path,
    bench: &str,
    extra: &[&str],
) -> Result<(), CommandError> {
    let target_dir_value = target_dir.display().to_string();
    let mut cmd = CommandIn::new(target, "rustup").map_err(|e| CommandError::SpawnFailed {
        label: "cargo bench --no-run".to_owned(),
        source: Box::new(e),
    })?;
    let _ = cmd.inherit_env();
    append_compile_args(&mut cmd, bench, &target_dir_value, extra);
    run_with_status(&cmd, "cargo bench --no-run")
}

fn append_compile_args<'a>(
    cmd: &mut CommandIn<'a>,
    bench: &'a str,
    target_dir: &'a str,
    extra: &'a [&'a str],
) {
    let _ = cmd.arg("run").arg(RUSTUP_TOOLCHAIN).arg("cargo").arg("bench");
    let _ = cmd.arg("--bench").arg(bench);
    let _ = cmd.args(&["--all-features"]).arg("--no-run");
    let _ = cmd.env("CARGO_TARGET_DIR", target_dir).args(extra);
}

/// `perf stat -x, -e instructions -- rustup run nightly-… cargo bench -- --bench`
///
/// # Errors
///
/// Returns an error when the perf command cannot be prepared, spawned, or exits unsuccessfully.
fn run_perf_stat(
    target: &TargetProject,
    target_dir: &Path,
    bench: &str,
    log_file: &Path,
) -> Result<(), CommandError> {
    let target_dir_value = target_dir.display().to_string();
    let log_file_value = log_file.display().to_string();
    let mut cmd = CommandIn::new(target, "perf").map_err(|e| CommandError::SpawnFailed {
        label: "perf stat cargo bench".to_owned(),
        source: Box::new(e),
    })?;
    let _ = cmd.inherit_env();
    append_perf_args(&mut cmd, bench, &target_dir_value, &log_file_value);
    run_with_status(&cmd, "perf stat cargo bench")
}

fn append_perf_args<'a>(
    cmd: &mut CommandIn<'a>,
    bench: &'a str,
    target_dir: &'a str,
    log_file: &'a str,
) {
    let _ = cmd.args(&["stat", "-x,", "-e", "instructions", "-o"]).arg(log_file);
    let _ = cmd.arg("--").arg("rustup").arg("run").arg(RUSTUP_TOOLCHAIN);
    let _ = cmd.arg("cargo").arg("bench").arg("--bench").arg(bench);
    let _ = cmd.args(&["--all-features"]);
    let _ = cmd.arg("--").arg("--bench").env("CARGO_TARGET_DIR", target_dir);
}
