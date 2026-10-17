// Fixture: triggers ARCHITECTURE_IMPORT_CORE_PROCESS for fully-qualified paths.
// Core/domain code spawning a subprocess with no `use` import.

pub fn list_dir() -> std::io::Result<std::process::Child> {
    std::process::Command::new("ls").spawn()
}
