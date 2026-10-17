
pub struct ShellPort;

impl ShellPort {
    pub fn run(&self, program: &str) -> Option<String> {
        let _ = program;
        None
    }
}
