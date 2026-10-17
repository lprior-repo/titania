
use std::env::var;
use std::fs::read_to_string;
use std::net::TcpStream;

pub fn load_config() -> std::io::Result<(String, TcpStream)> {
    let path = var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml"));
    let config = read_to_string(path)?;
    TcpStream::connect("127.0.0.1:80").map(|stream| (config, stream))
}
