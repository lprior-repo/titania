// Fixture: triggers ARCHITECTURE_IMPORT_CORE_FS when forbidden grouped member is not first.
use std::{collections::HashMap, fs};

pub fn load_config(path: &str) -> std::io::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect())
}
