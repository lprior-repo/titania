
pub fn list_dir() -> std::io::Result<std::process::Child> {
    std::process::Command::new("ls").spawn()
}
