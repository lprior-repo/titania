use std :: fs;

pub fn load() {
    let _c = fs::read_to_string("config.toml");
}
