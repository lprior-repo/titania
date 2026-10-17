
pub fn load_config() -> String {
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml"));
    std::fs::read_to_string(path).unwrap_or_default()
}

pub fn peer() -> std::io::Result<std::net::TcpStream> {
    std::net::TcpStream::connect("127.0.0.1:80")
}
