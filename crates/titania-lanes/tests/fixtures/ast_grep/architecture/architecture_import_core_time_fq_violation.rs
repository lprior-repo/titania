// Fixture: triggers ARCHITECTURE_IMPORT_CORE_TIME for fully-qualified paths.
// Core/domain code reading the wall clock with no `use` import.

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

pub fn started() -> std::time::Instant {
    std::time::Instant::now()
}
