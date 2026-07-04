// Fixture: allowed — returns strings and typed errors through explicit imports.

use std::collections::HashMap;

pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

pub fn report_error(msg: &str) -> std::result::Result<(), std::io::Error> {
    Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
}

pub fn get_config_value(
    key: &str,
    defaults: &HashMap<String, String>,
) -> std::result::Result<String, &'static str> {
    defaults
        .get(key)
        .cloned()
        .ok_or("config key not found")
}
