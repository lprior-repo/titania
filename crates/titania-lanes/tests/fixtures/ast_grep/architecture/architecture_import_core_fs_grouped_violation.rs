// Fixture: triggers ARCHITECTURE_IMPORT_CORE_FS for grouped std imports.
// Core/domain code importing direct filesystem/environment/network APIs.
use std::{env, fs, net};

pub fn load_config() -> std::io::Result<(String, net::TcpStream)> {
    let path = env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml"));
    let config = fs::read_to_string(path)?;
    let stream = net::TcpStream::connect("127.0.0.1:80")?;
    Ok((config, stream))
}
