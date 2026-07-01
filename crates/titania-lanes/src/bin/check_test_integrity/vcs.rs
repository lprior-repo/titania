use std::{env, fs};

use thiserror::Error;
use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneError, OutputStream};

use super::{RootInfo, Vcs, scan::is_test_path};

/// VCS-layer errors. The bin orchestrator (`mod.rs::check`) wraps these
/// in its own `TestIntegrityError` via `#[from]` and prints the `Display`
/// impl on failure. The structured variants survive end-to-end so callers
/// can match on `Spawn` / `Exit` / `EmptyBase` / `InvalidBase`.
///
/// `Spawn` and `Exit` carry the `LaneError` source so the underlying
/// `CommandIn` failure mode (I/O, non-zero exit, non-UTF-8 output) is
/// preserved rather than collapsed to a string at the bin boundary.
#[derive(Debug, Error)]
pub(super) enum VcsError {
    #[error("{tool} {args} failed: {source}")]
    Spawn {
        tool: String,
        args: String,
        #[source]
        source: LaneError,
    },
    #[error("{tool} {args} failed: {stderr}")]
    Exit { tool: String, args: String, stderr: String },
    #[error("empty base revision")]
    EmptyBase,
    #[error("invalid base revision {base:?}: {source}")]
    InvalidBase {
        base: String,
        #[source]
        source: Box<VcsError>,
    },
}

pub(super) type VcsResult<T> = Result<T, VcsError>;

fn spawn<'a, 'b>(
    tool: &'a str,
    args: &'b [&'b str],
    target: &'a TargetProject,
) -> VcsResult<CommandIn<'a>>
where
    'b: 'a,
{
    let joined_args = args.join(" ");
    let mut command = CommandIn::new(target, tool).map_err(|source| VcsError::Spawn {
        tool: tool.to_owned(),
        args: joined_args,
        source,
    })?;
    command.inherit_env();
    command.args(args);
    Ok(command)
}

fn tool_output(tool: &str, args: &[&str], target: &TargetProject) -> VcsResult<String> {
    let joined_args = args.join(" ");
    let output = spawn(tool, args, target)?.run_capture_raw().map_err(|source| {
        VcsError::Spawn { tool: tool.to_owned(), args: joined_args.clone(), source }
    })?;
    if output.status.success() {
        Ok(output
            .stdout_str()
            .map(str::to_owned)
            .map_err(|_| VcsError::Spawn {
                tool: tool.to_owned(),
                args: joined_args,
                source: LaneError::NonUtf8Output {
                    program: tool.to_owned(),
                    stream: OutputStream::Stdout,
                },
            })?
            .to_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(VcsError::Exit { tool: tool.to_owned(), args: joined_args, stderr })
    }
}

fn command_output(args: &[&str], target: &TargetProject) -> VcsResult<String> {
    tool_output("git", args, target)
}

fn jj_output(args: &[&str], target: &TargetProject) -> VcsResult<String> {
    tool_output("jj", args, target)
}

fn command_output_allow_fail(args: &[&str], target: &TargetProject) -> Option<String> {
    let mut command = CommandIn::new(target, "git").ok()?;
    command.inherit_env();
    command.args(args);
    command
        .run_capture_raw()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| output.stdout_str().ok().map(str::to_owned))
}

pub(super) fn root_dir(target: &TargetProject) -> VcsResult<RootInfo> {
    if command_output(&["rev-parse", "--show-toplevel"], target).is_ok() {
        return Ok(RootInfo { vcs: Vcs::Git });
    }
    jj_output(&["workspace", "root"], target).map(|_| RootInfo { vcs: Vcs::Jj })
}

pub(super) fn default_base(target: &TargetProject, vcs: Vcs) -> String {
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
    match command_output_allow_fail(&["merge-base", "origin/main", "HEAD"], target)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
    {
        Some(base) => base,
        None => "HEAD".to_owned(),
    }
}

pub(super) fn validate_base_revision(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> VcsResult<()> {
    if base.trim().is_empty() {
        return Err(VcsError::EmptyBase);
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
    result.map_err(|error| VcsError::InvalidBase { base: base.to_owned(), source: Box::new(error) })
}

pub(super) fn changed_files(
    target: &TargetProject,
    base: &str,
    vcs: Vcs,
) -> VcsResult<Vec<(String, String)>> {
    match vcs {
        Vcs::Git => git_changed_files(target, base),
        Vcs::Jj => jj_changed_files(target, base),
    }
}

fn git_changed_files(target: &TargetProject, base: &str) -> VcsResult<Vec<(String, String)>> {
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

fn jj_changed_files(target: &TargetProject, base: &str) -> VcsResult<Vec<(String, String)>> {
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

pub(super) fn diff_text(target: &TargetProject, base: &str, vcs: Vcs) -> VcsResult<String> {
    match vcs {
        Vcs::Git => git_diff_text(target, base),
        Vcs::Jj => jj_output(&["diff", "--git", "--from", base, "--to", "@"], target),
    }
}

fn git_diff_text(target: &TargetProject, base: &str) -> VcsResult<String> {
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

fn untracked_files(target: &TargetProject, vcs: Vcs) -> VcsResult<Vec<String>> {
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

fn untracked_file_diff(target: &TargetProject, path: &str) -> Option<String> {
    let full_path = target.as_std_path().join(path);
    let content = fs::read_to_string(full_path).ok()?;
    let additions = content.lines().map(|line| format!("+{line}\n")).collect::<String>();
    Some(format!(
        "diff --git a/{path} b/{path}\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n{additions}",
        content.lines().count()
    ))
}
