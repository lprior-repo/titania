use std::{env, fmt::Write as _, fs};

use titania_core::TargetProject;
use titania_lanes::{CommandIn, CommandOutput};

use super::{ChangedFile, ChangedFiles, RootInfo, TestIntegrityError, Vcs, scan::is_test_path};

/// Capture UTF-8 stdout from a VCS tool command.
///
/// # Errors
///
/// Returns command construction, spawn, non-zero exit, or non-UTF8 stdout
/// failures.
fn tool_output(
    tool: &str,
    args: &[&str],
    target: &TargetProject,
) -> Result<String, TestIntegrityError> {
    let joined_args = args.join(" ");
    let mut command = CommandIn::new(target, tool)
        .map_err(|error| format!("{tool} {joined_args} failed to start: {error}"))?;
    let _ = command.inherit_env();
    let _ = command.args(args);
    let output = command
        .run_capture_raw()
        .map_err(|error| format!("{tool} {joined_args} failed to start: {error}"))?;
    if output.status().success() {
        vcs_stdout_or_error(tool, &joined_args, &output)
    } else {
        let stderr = String::from_utf8_lossy(output.stderr());
        Err(TestIntegrityError::from(format!("{tool} {joined_args} failed: {stderr}")))
    }
}

/// Extract UTF-8 stdout from a VCS command output, converting non-UTF8 to an error.
///
/// # Errors
///
/// Returns [`TestIntegrityError`] when stdout is not valid UTF-8.
fn vcs_stdout_or_error(
    tool: &str,
    joined_args: &str,
    output: &CommandOutput,
) -> Result<String, TestIntegrityError> {
    let bytes = output.stdout();
    std::str::from_utf8(bytes).map(str::to_owned).map_err(|error| {
        TestIntegrityError::from(format!("{tool} {joined_args} returned non-UTF8 stdout: {error}"))
    })
}

/// Capture UTF-8 stdout from `git`.
///
/// # Errors
///
/// Returns command construction, spawn, non-zero exit, or non-UTF8 stdout
/// failures.
fn command_output(args: &[&str], target: &TargetProject) -> Result<String, TestIntegrityError> {
    tool_output("git", args, target)
}

/// Capture UTF-8 stdout from `jj`.
///
/// # Errors
///
/// Returns command construction, spawn, non-zero exit, or non-UTF8 stdout
/// failures.
fn jj_output(args: &[&str], target: &TargetProject) -> Result<String, TestIntegrityError> {
    tool_output("jj", args, target)
}

fn command_output_allow_fail(args: &[&str], target: &TargetProject) -> Option<String> {
    let Ok(mut command) = CommandIn::new(target, "git") else {
        return None;
    };
    let _ = command.inherit_env();
    let _ = command.args(args);
    command
        .run_capture_raw()
        .ok()
        .filter(|output| output.status().success())
        .and_then(|output| output.stdout_str().ok().map(str::to_owned))
}

/// Resolve the active repository VCS.
///
/// # Errors
///
/// Returns an error when neither `git` nor `jj` can resolve a workspace root.
pub(super) fn root_dir(target: &TargetProject) -> Result<RootInfo, TestIntegrityError> {
    if command_output(&["rev-parse", "--show-toplevel"], target).is_ok() {
        return Ok(RootInfo { vcs: Vcs::Git });
    }
    jj_output(&["workspace", "root"], target).map(|_| RootInfo { vcs: Vcs::Jj })
}

pub(super) fn default_base(target: &TargetProject, vcs: Vcs) -> String {
    if let Some(value) = env_base() {
        return value;
    }
    if vcs == Vcs::Jj {
        return "@-".to_owned();
    }
    let dirty = command_output(&["status", "--porcelain"], target)
        .map_or(true, |text| !text.trim().is_empty());
    if dirty {
        return "HEAD".to_owned();
    }
    let Some(base) = command_output_allow_fail(&["merge-base", "origin/main", "HEAD"], target)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
    else {
        return "HEAD".to_owned();
    };
    base
}

fn env_base() -> Option<String> {
    env::var("TEST_INTEGRITY_BASE").ok().filter(|value| !value.trim().is_empty())
}

/// Validate that the selected base revision exists.
///
/// # Errors
///
/// Returns an error when the base is empty or the VCS cannot resolve it.
pub(super) fn validate_base_revision(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> Result<(), TestIntegrityError> {
    if base.trim().is_empty() {
        return Err(TestIntegrityError::from("empty base revision"));
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
    result.map_err(|error| {
        TestIntegrityError::from(format!("invalid base revision {base:?}: {error}"))
    })
}

/// Return changed files since the selected base revision.
///
/// # Errors
///
/// Returns VCS command failures from the selected backend.
pub(super) fn changed_files(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> Result<ChangedFiles, TestIntegrityError> {
    match vcs {
        Vcs::Git => git_changed_files(target, base),
        Vcs::Jj => jj_changed_files(target, base),
    }
}

/// Return git changed files plus untracked files.
///
/// # Errors
///
/// Returns git command failures from diff or untracked-file discovery.
fn git_changed_files(
    target: &TargetProject,
    base: &str,
) -> Result<ChangedFiles, TestIntegrityError> {
    let mut entries = parse_git_name_status(&command_output(
        &["diff", "--name-status", "--find-renames", base, "--"],
        target,
    )?);
    entries
        .extend(untracked_files(target, Vcs::Git)?.into_iter().map(|path| ("??".to_owned(), path)));
    Ok(entries)
}

fn parse_git_name_status(text: &str) -> ChangedFiles {
    text.lines().filter_map(parse_git_name_status_line).collect()
}

fn parse_git_name_status_line(line: &str) -> Option<ChangedFile> {
    let parts = line.split('\t').collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let status = parts.first().copied().map_or("", |value| value).to_owned();
    let path = parts.last().copied().map_or("", |value| value).to_owned();
    Some((status, path))
}

/// Return jj changed files since the selected base revision.
///
/// # Errors
///
/// Returns jj command failures.
fn jj_changed_files(
    target: &TargetProject,
    base: &str,
) -> Result<ChangedFiles, TestIntegrityError> {
    jj_output(&["diff", "--summary", "--from", base, "--to", "@"], target)
        .map(|text| text.lines().filter_map(parse_jj_summary_line).collect())
}

fn parse_jj_summary_line(line: &str) -> Option<ChangedFile> {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    let status = chars.next()?.to_string();
    let path = trimmed.get(1..)?.trim().to_owned();
    if path.is_empty() {
        return None;
    }
    Some((status, path))
}

/// Return unified diff text since the selected base revision.
///
/// # Errors
///
/// Returns VCS command failures from the selected backend.
pub(super) fn diff_text(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> Result<String, TestIntegrityError> {
    match vcs {
        Vcs::Git => git_diff_text(target, base),
        Vcs::Jj => jj_output(&["diff", "--git", "--from", base, "--to", "@"], target),
    }
}

/// Return git diff text plus synthesized diffs for untracked test files.
///
/// # Errors
///
/// Returns git diff or untracked-file discovery failures.
fn git_diff_text(target: &TargetProject, base: &str) -> Result<String, TestIntegrityError> {
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

/// Return untracked files visible to the selected VCS.
///
/// # Errors
///
/// Returns git command failures for git workspaces.
fn untracked_files(target: &TargetProject, vcs: Vcs) -> Result<Vec<String>, TestIntegrityError> {
    match vcs {
        Vcs::Git => command_output(&["ls-files", "--others", "--exclude-standard"], target)
            .map(|text| parse_untracked_files(&text)),
        Vcs::Jj => Ok(Vec::new()),
    }
}

fn parse_untracked_files(text: &str) -> Vec<String> {
    text.lines().map(str::trim).filter(|path| !path.is_empty()).map(str::to_owned).collect()
}

fn untracked_file_diff(target: &TargetProject, path: &str) -> Option<String> {
    let full_path = target.as_std_path().join(path);
    let content = fs::read_to_string(full_path).ok()?;
    let additions = content
        .lines()
        .try_fold(String::new(), |mut output, line| {
            writeln!(&mut output, "+{line}").map(|()| output)
        })
        .ok()?;
    let mut diff = String::new();
    write!(
        &mut diff,
        "diff --git a/{path} b/{path}\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n{additions}",
        content.lines().count()
    )
    .ok()?;
    Some(diff)
}
