// Fixture: triggers ARCHITECTURE_IMPORT_CORE_FS through a local macro.
// A `macro_rules!` body that expands to direct filesystem I/O cannot
// bypass capability policy: detectors scan macro bodies as code text.

macro_rules! load_config {
    () => {
        std::fs::read_to_string("config.toml").unwrap_or_default()
    };
}

pub fn load() -> String {
    load_config!()
}
