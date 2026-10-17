// Fixture: triggers ARCHITECTURE_IMPORT_CORE_FS for fully-qualified paths.
// Core/domain code using filesystem/environment/network APIs with no `use`
// import — the alias/re-export bypass this bead closes.

pub fn load_config() -> String {
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml"));
    std::fs::read_to_string(path).unwrap_or_default()
}

pub fn peer() -> std::io::Result<std::net::TcpStream> {
    std::net::TcpStream::connect("127.0.0.1:80")
}
