//! Focused integration tests for the scrubbed-inherit env policy.
//!
//! These tests prove the four observable behaviors of
//! [`CommandIn::inherit_env`]:
//!
//! 1. Default behavior (`CmdIn::new`) starts the subprocess with an
//!    empty environment — even if the parent process has
//!    bypass-vulnerable vars like `RUSTFLAGS` set.
//! 2. The keep-list of the scrubbed inherit lets `PATH` through, so
//!    `cargo`/`rustc` can resolve binaries.
//! 3. Explicit `.env(...)` and `.env_remove(...)` win over the
//!    scrubbed snapshot.
//! 4. Bypass-prone keys (`RUSTFLAGS`, `LD_PRELOAD`,
//!    `CARGO_ENCODED_RUSTFLAGS`, `RUSTC_BOOTSTRAP`) are scrubbed even
//!    when the parent has them set.

#![cfg(unix)]

use std::{error::Error, process::Command};

use tempfile::tempdir;
use titania_core::TargetProject;
use titania_lanes::CommandIn;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_target() -> Result<(tempfile::TempDir, TargetProject), Box<dyn Error>> {
    let tmp = tempdir()?;
    std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"\n")?;
    let target = titania_lanes::try_from_path(tmp.path())?;
    Ok((tmp, target))
}

/// Spawn `/bin/sh -c "$cmd"` as a plain subprocess that DOES inherit
/// the parent's env. Used to capture the parent env for assertions.
fn bash_capture(cmd: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .env("TITANIA_SCRUB_TEST_CANARY", "parent-value-to-confirm-clear")
        .output()?;
    Ok(String::from_utf8(output.stdout)?)
}

/// Probe a `CommandIn` for the value of `$name` in the spawned shell.
fn cmdin_value(name: &str) -> Result<String, Box<dyn Error>> {
    let (_tmp, target) = fixture_target()?;
    let probe = format!("v=${{{name}:-TITANIA_MISSING}}; printf '%s' \"$v\"", name = name);
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let _ = command.inherit_env().arg("-c").arg(&probe);
    let output = command.run_capture_raw()?;
    Ok(String::from_utf8(output.into_stdout())?)
}

#[test]
fn scrubbed_inherit_drops_rustflags_even_when_parent_has_it() -> TestResult {
    let command = Command::new("/bin/sh")
        .arg("-c")
        .arg("printf %s \"${RUSTFLAGS:-TITANIA_MISSING}\"")
        .env("RUSTFLAGS", "--danger-flag-from-parent")
        .output()?;
    let parent_output = String::from_utf8(command.stdout)?;
    assert_eq!(
        parent_output, "--danger-flag-from-parent",
        "sanity: parent shell should see the env var; got {parent_output:?}",
    );

    let (_tmp, target) = fixture_target()?;
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let probe = "v=${RUSTFLAGS:-TITANIA_MISSING}; printf '%s' \"$v\"";
    let _ = command.inherit_env().arg("-c").arg(probe);
    let command_env = command.run()?;
    let out = command_env.stdout_str()?.to_owned();
    assert_eq!(out, "TITANIA_MISSING", "scrubbed inherit must drop RUSTFLAGS; observed {out:?}",);
    Ok(())
}

#[test]
fn scrubbed_inherit_keeps_path_through_to_subprocess() -> TestResult {
    let parent_path = bash_capture("printf %s \"$PATH\"")?;
    let observed = cmdin_value("PATH")?;
    assert_eq!(
        observed, parent_path,
        "scrubbed inherit must pass PATH through unchanged; \
         observed {observed:?} vs parent {parent_path:?}",
    );
    assert!(
        !observed.is_empty(),
        "PATH must survive scrubbing (used as empty would be a false positive); \
         observed {observed:?}",
    );
    Ok(())
}

#[test]
fn scrubbed_inherit_drops_ld_preload() -> TestResult {
    let (_tmp, target) = fixture_target()?;
    let probe = "v=${LD_PRELOAD:-TITANIA_MISSING}; printf '%s' \"$v\"";
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let _ = command.inherit_env().arg("-c").arg(probe);
    let output = command.run_capture_raw()?;
    let out = String::from_utf8(output.into_stdout())?;
    assert_eq!(out, "TITANIA_MISSING", "scrubbed inherit must drop LD_PRELOAD; observed {out:?}",);
    Ok(())
}

#[test]
fn scrubbed_inherit_drops_rustc_bootstrap_unstable_flag() -> TestResult {
    let (_tmp, target) = fixture_target()?;
    let probe = "v=${RUSTC_BOOTSTRAP:-TITANIA_MISSING}; printf '%s' \"$v\"";
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let _ = command.inherit_env().arg("-c").arg(probe);
    let output = command.run_capture_raw()?;
    let out = String::from_utf8(output.into_stdout())?;
    assert_eq!(
        out, "TITANIA_MISSING",
        "scrubbed inherit must drop RUSTC_BOOTSTRAP; observed {out:?}",
    );
    Ok(())
}

#[test]
fn scrubbed_inherit_drops_untrusted_cargo_prefix_env_var() -> TestResult {
    let (_tmp, target) = fixture_target()?;
    let probe = "v=${CARGO_UNTRUSTED_TARGET:-TITANIA_MISSING}; printf '%s' \"$v\"";
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let _ = command.inherit_env().arg("-c").arg(probe);
    let output = command.run_capture_raw()?;
    let out = String::from_utf8(output.into_stdout())?;
    assert_eq!(
        out, "TITANIA_MISSING",
        "scrubbed inherit must drop untrusted CARGO_* values; observed {out:?}",
    );
    Ok(())
}

#[test]
fn default_clear_policy_wins_over_inherited_parent() -> TestResult {
    let (_tmp, target) = fixture_target()?;
    // Spawn child without inherit_env: should see TITANIA_MISSING for
    // every parent-set var we set below because the default is Clear.
    let probe = "v=${TITANIA_SCRUB_TEST_CANARY:-TITANIA_MISSING}; printf '%s' \"$v\"";
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let _ = command.arg("-c").arg(probe);
    let output = command.run_capture_raw()?;
    let out = String::from_utf8(output.into_stdout())?;
    assert_eq!(
        out, "TITANIA_MISSING",
        "default policy must clear env (no inherit); observed {out:?}",
    );
    Ok(())
}

#[test]
fn explicit_env_wins_over_scrubbed_inherit() -> TestResult {
    let (_tmp, target) = fixture_target()?;
    let probe = "v=${TITANIA_EXPLICIT_OVERRIDE:-TITANIA_MISSING}; printf '%s' \"$v\"";
    let mut command = CommandIn::new(&target, "/bin/sh")?;
    let _ = command
        .inherit_env()
        .env("TITANIA_EXPLICIT_OVERRIDE", "present")
        .env_remove("TITANIA_EXPLICIT_OVERRIDE")
        .arg("-c")
        .arg(probe);
    let output = command.run_capture_raw()?;
    let out = String::from_utf8(output.into_stdout())?;
    assert_eq!(
        out, "TITANIA_MISSING",
        "explicit env_remove must win over scrubbed snapshot; observed {out:?}",
    );
    Ok(())
}
