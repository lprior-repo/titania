pub fn load() -> String {
    std::fs::read_to_string("config.toml").unwrap_or_default()
}

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

pub fn peer() -> String {
    std::net::TcpStream::connect("127.0.0.1:80").map(|_| String::new()).unwrap_or_default()
}
