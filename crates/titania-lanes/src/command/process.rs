use std::process::{Child, Command, Stdio};

use super::LaneError;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
pub(super) fn configure_process_group(cmd: &mut Command) {
    let _ = cmd.process_group(0);
}

#[cfg(not(unix))]
pub(super) fn configure_process_group(_cmd: &mut Command) {}

/// Terminate the process group and then the direct child process.
///
/// # Errors
/// Returns [`LaneError::Io`] if killing the direct child process fails.
pub(super) fn terminate_child_tree(child: &mut Child, program: String) -> Result<(), LaneError> {
    terminate_process_group(child.id());
    child.kill().map_err(|source| LaneError::Io { program, source })
}

#[cfg(unix)]
fn terminate_process_group(child_id: u32) {
    let group = format!("-{child_id}");
    let status = Command::new("/bin/kill")
        .arg("-TERM")
        .arg(group)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(_) | Err(_) => (),
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_child_id: u32) {}
