
use std::fs as filesystem;
use std::{env as environment, net};

pub fn load_config() -> String {
    let path = environment::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml"));
    filesystem::read_to_string(path).unwrap_or_default()
}

pub fn peer() -> std::io::Result<net::TcpStream> {
    net::TcpStream::connect("127.0.0.1:80")
}
