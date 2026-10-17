// Fixture: allowed — no process spawning in core/domain.
// Core code describes work; the shell spawns processes through a typed port.

pub struct ShellPort;

impl ShellPort {
    pub fn run(&self, program: &str) -> Option<String> {
        // Real spawning happens in the shell; core only names the request.
        let _ = program;
        None
    }
}
