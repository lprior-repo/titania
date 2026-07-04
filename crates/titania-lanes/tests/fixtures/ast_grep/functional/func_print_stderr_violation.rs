// Fixture: triggers FUNC_PRINT_STDERR
// eprintln! calls in production source.

pub fn report_error(msg: &str) {
    eprintln!("Error: {msg}");
}

pub fn warn_deprecated(feature: &str) {
    eprintln!("Deprecated feature used: {feature}");
}
