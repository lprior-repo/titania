// Fixture: triggers FUNC_RESULT_STRING
// Result<T, String> error variant in production source — should use typed error types.

pub fn parse_config(input: &str) -> Result<ConfigData, String> {
    let value = input.trim();
    if value.is_empty() {
        return Err("config value is empty".to_string());
    }
    let data = ConfigData { value: value.to_string() };
    Ok(data)
}

pub fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

pub struct ConfigData {
    pub value: String,
}
