
use std::process::Command;

pub fn list_dir() -> std::io::Result<std::process::Child> {
    Command::new("ls").spawn()
}
