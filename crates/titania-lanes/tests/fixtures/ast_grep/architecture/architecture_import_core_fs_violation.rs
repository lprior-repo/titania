
use std::fs::*;

pub fn read_config() -> String {
    let content = read_to_string("config.toml").unwrap();
    content
}
