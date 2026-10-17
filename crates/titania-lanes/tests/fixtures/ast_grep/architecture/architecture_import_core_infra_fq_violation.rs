// Fixture: triggers ARCHITECTURE_IMPORT_CORE_INFRA for fully-qualified paths.
// Core/domain code reaching the async runtime with no `use` import.

pub fn spawn_background() {
    tokio::spawn(async { 1 });
}

pub fn client() -> reqwest::Client {
    reqwest::Client::new()
}
