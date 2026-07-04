// Fixture: triggers FUNC_PRINT_STDOUT
// Print calls to stdout in production source.

pub fn greet(name: &str) {
    println!("Hello, {name}!");
    print!("Welcome to the system");
}

pub fn log_progress(current: usize, total: usize) {
    print!("\rProgress: {current}/{total}");
}
