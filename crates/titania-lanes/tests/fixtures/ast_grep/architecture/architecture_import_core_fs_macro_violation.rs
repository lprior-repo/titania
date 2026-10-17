
macro_rules! load_config {
    () => {
        std::fs::read_to_string("config.toml").unwrap_or_default()
    };
}

pub fn load() -> String {
    load_config!()
}
