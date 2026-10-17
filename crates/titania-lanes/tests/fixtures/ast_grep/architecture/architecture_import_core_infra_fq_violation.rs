
pub fn spawn_background() {
    tokio::spawn(async { 1 });
}

pub fn client() -> reqwest::Client {
    reqwest::Client::new()
}
