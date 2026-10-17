use std::{env, fs, net};

pub fn load_config() -> std::io::Result<(String, net::TcpStream)> {
    let path = env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml"));
    let config = fs::read_to_string(path)?;
    let stream = net::TcpStream::connect("127.0.0.1:80")?;
    Ok((config, stream))
}
