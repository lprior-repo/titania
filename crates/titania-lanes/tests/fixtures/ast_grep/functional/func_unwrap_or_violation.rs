// Fixture: triggers FUNC_UNWRAP_OR
// .unwrap_or calls in production source — should use proper error handling.

pub fn get_config_value(key: &str, defaults: &std::collections::HashMap<String, String>) -> String {
    defaults.get(key).cloned().unwrap_or("unknown".to_string())
}

pub fn parse_port(s: &str) -> u16 {
    s.parse::<u16>().unwrap_or(8080)
}

pub fn first_element<'a>(items: &'a [String]) -> &'a str {
    items.first().unwrap_or(&String::new())
}
