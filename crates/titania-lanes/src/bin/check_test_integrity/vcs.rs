use std::{env, fmt::Write, fs};

use titania_core::TargetProject;
use titania_lanes::CommandIn;

use super::{RootInfo, Vcs, scan::is_test_path};

/// Runs `tool` with `args` in the target project and returns its stdout.
///
/// # Errors
/// Returns `Err(String)` when `CommandIn::new` fails to spawn the tool,
/// when `run_capture_raw` fails, or when stdout is not valid UTF-8.
fn tool_output(tool: &str, args: &[&str], target: &TargetProject) -> Result<String, String> {
    let joined_args = args.join(" ");
    let mut command = CommandIn::new(target, tool)
        .map_err(|error| format!("{tool} {joined_args} failed to start: {error}"))?;
    command.inherit_env();
    command.args(args);
    let output = command
        .run_capture_raw()
        .map_err(|error| format!("{tool} {joined_args} failed to start: {error}"))?;
    if output.status.success() {
        output
            .stdout_str()
            .map(str::to_owned)
            .map_err(|error| format!("{tool} {joined_args} returned non-UTF8 stdout: {error}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{tool} {joined_args} failed: {stderr}"))
    }
}

/// Runs `git` with `args` and returns its stdout.
///
/// # Errors
/// Returns `Err(String)` when `git` cannot be spawned, exits non-zero,
/// or returns non-UTF-8 stdout.
fn command_output(args: &[&str], target: &TargetProject) -> Result<String, String> {
    tool_output("git", args, target)
}

/// Runs `jj` with `args` and returns its stdout.
///
/// # Errors
/// Returns `Err(String)` when `jj` cannot be spawned, exits non-zero,
/// or returns non-UTF-8 stdout.
fn jj_output(args: &[&str], target: &TargetProject) -> Result<String, String> {
    tool_output("jj", args, target)
}

fn command_output_allow_fail(args: &[&str], target: &TargetProject) -> Option<String> {
    let Ok(mut command) = CommandIn::new(target, "git") else { return None };
    command.inherit_env();
    command.args(args);
    command
        .run_capture_raw()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| output.stdout_str().ok().map(str::to_owned))
}

/// Detects the version-control system root and returns its type.
///
/// # Errors
/// Returns `Err(String)` when neither `git rev-parse` nor `jj workspace root`
/// can execute successfully.
pub fn root_dir(target: &TargetProject) -> Result<RootInfo, String> {
    if command_output(&["rev-parse", "--show-toplevel"], target).is_ok() {
        return Ok(RootInfo { vcs: Vcs::Git });
    }
    jj_output(&["workspace", "root"], target).map(|_| RootInfo { vcs: Vcs::Jj })
}

/// Returns the default base revision string for the target VCS.
pub fn default_base(target: &TargetProject, vcs: Vcs) -> String {
    if let Ok(value) = env::var("TEST_INTEGRITY_BASE") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if vcs == Vcs::Jj {
        return "@-".to_owned();
    }
    let dirty = command_output(&["status", "--porcelain"], target)
        .map_or(true, |text| !text.trim().is_empty());
    if dirty {
        return "HEAD".to_owned();
    }
    command_output_allow_fail(&["merge-base", "origin/main", "HEAD"], target)
        .map_or_else(
            || "HEAD".to_owned(),
            |text| text.trim().to_owned(),
        )
}

/// Validates that `base` is a parseable revision for the given VCS.
///
/// # Errors
/// Returns `Err(String)` when `base` is empty, when `git rev-parse`
/// (or `jj log`) exits non-zero, or when command execution fails.
pub fn validate_base_revision(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> Result<(), String> {
    if base.trim().is_empty() {
        return Err("empty base revision".to_owned());
    }
    let result = match vcs {
        Vcs::Git => {
            let commit = format!("{base}^{{commit}}");
            command_output(&["rev-parse", "--verify", &commit], target).map(|_| ())
        }
        Vcs::Jj => {
            jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], target).map(|_| ())
        }
    };
    result.map_err(|error| format!("invalid base revision {base:?}: {error}"))
}

/// Returns the changed files between `base` and the working tree.
///
/// # Errors
/// Returns `Err(String)` when the underlying VCS command fails.
pub fn changed_files(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> Result<Vec<(String, String)>, String> {
    match vcs {
        Vcs::Git => git_changed_files(target, base),
        Vcs::Jj => jj_changed_files(target, base),
    }
}

/// Returns the changed files for a Git repository between `base` and the working tree.
///
/// # Errors
/// Returns `Err(String)` when `git diff` or `git ls-files` fails,
/// or when command execution fails.
fn git_changed_files(target: &TargetProject, base: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = parse_git_name_status(&command_output(
        &["diff", "--name-status", "--find-renames", base, "--"],
        target,
    )?);
    entries
        .extend(untracked_files(target, Vcs::Git)?.into_iter().map(|path| ("??".to_owned(), path)));
    Ok(entries)
}

fn parse_git_name_status(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let parts = line.split('\t').collect::<Vec<_>>();
            (parts.len() >= 2).then(|| {
                let status = parts.first().copied().map_or("", |value| value).to_owned();
                let path = parts.last().copied().map_or("", |value| value).to_owned();
                (status, path)
            })
        })
        .collect()
}

/// Returns the changed files for a Jujutsu repository between `base` and the working tree.
///
/// # Errors
/// Returns `Err(String)` when `jj diff` fails or command execution fails.
fn jj_changed_files(target: &TargetProject, base: &str) -> Result<Vec<(String, String)>, String> {
    jj_output(&["diff", "--summary", "--from", base, "--to", "@"], target).map(|text| {
        text.lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                let mut chars = trimmed.chars();
                let status = chars.next()?.to_string();
                let path = trimmed.get(1..)?.trim().to_owned();
                (!path.is_empty()).then_some((status, path))
            })
            .collect()
    })
}

/// Returns the diff text between `base` and the working tree.
///
/// # Errors
/// Returns `Err(String)` when the underlying VCS command fails.
pub fn diff_text(target: &TargetProject, base: &str, vcs: Vcs) -> Result<String, String> {
    match vcs {
        Vcs::Git => git_diff_text(target, base),
        Vcs::Jj => jj_output(&["diff", "--git", "--from", base, "--to", "@"], target),
    }
}

/// Returns the Git diff between `base` and the working tree, including
/// synthetic diffs for untracked test files.
///
/// # Errors
/// Returns `Err(String)` when `git diff` or `git ls-files` fails,
/// or when command execution fails.
fn git_diff_text(target: &TargetProject, base: &str) -> Result<String, String> {
    let mut text = command_output(&["diff", "--find-renames", "--unified=0", base, "--"], target)?;
    let untracked = untracked_files(target, Vcs::Git)?;
    let extra = untracked
        .iter()
        .filter(|path| is_test_path(path))
        .filter_map(|path| untracked_file_diff(target, path))
        .collect::<String>();
    text.push_str(&extra);
    Ok(text)
}

/// Returns the list of untracked files for the given VCS.
///
/// # Errors
/// Returns `Err(String)` when `git ls-files` fails (Jj always returns `Ok`).
fn untracked_files(target: &TargetProject, vcs: Vcs) -> Result<Vec<String>, String> {
    match vcs {
        Vcs::Git => {
            command_output(&["ls-files", "--others", "--exclude-standard"], target).map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
        }
        Vcs::Jj => Ok(Vec::new()),
    }
}

/// Generates a synthetic unified diff for an untracked file.
///
/// Returns `None` when the file cannot be read from the filesystem.
fn untracked_file_diff(target: &TargetProject, path: &str) -> Option<String> {
    let full_path = target.as_std_path().join(path);
    let content = fs::read_to_string(full_path).ok()?;
    let mut additions = String::new();
    for line in content.lines() {
        writeln!(&mut additions, "+{line}").ok()?;
    }
    Some(format!(
        "diff --git a/{path} b/{path}\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n{additions}",
        content.lines().count()
    ))
}
