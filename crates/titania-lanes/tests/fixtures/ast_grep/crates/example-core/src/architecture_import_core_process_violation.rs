// Fixture: triggers ARCHITECTURE_IMPORT_CORE_PROCESS.
// Core/domain code spawning a subprocess directly.

use std::process::Command;

pub fn list_dir() -> std::io::Result<std::process::Child> {
    Command::new("ls").spawn()
}
