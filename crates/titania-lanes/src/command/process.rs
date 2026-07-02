use std::process::{Child, Command};

use super::LaneError;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// Place the child into a new process group so we can terminate the whole
/// tree on timeout. Unix-only; on other platforms this is a no-op.
#[cfg(unix)]
pub(super) fn configure_process_group(cmd: &mut Command) {
    let _ = cmd.process_group(0);
}

/// Process-group configuration stub for non-Unix targets.
#[cfg(not(unix))]
pub(super) fn configure_process_group(_cmd: &mut Command) {}

/// Terminate the child's process group, then kill the child itself.
///
/// # Errors
/// Returns [`LaneError::Io`] if the direct `kill` syscall fails. The
/// best-effort group termination is fire-and-forget.
pub(super) fn terminate_child_tree(child: &mut Child, program: String) -> Result<(), LaneError> {
    terminate_process_group(child.id());
    child.kill().map_err(|source| LaneError::Io { program, source })
}

#[cfg(unix)]
fn terminate_process_group(child_id: u32) {
    let group = format!("-{child_id}");
    let status = Command::new("/bin/kill").arg("-TERM").arg(group).status();
    match status {
        Ok(_) | Err(_) => (),
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_child_id: u32) {}