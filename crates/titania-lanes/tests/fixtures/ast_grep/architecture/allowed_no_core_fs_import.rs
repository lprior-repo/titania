// Fixture: allowed — no direct filesystem I/O imports in core/domain.
// Core code uses an abstracted config service instead.

pub struct ConfigService;

impl ConfigService {
    pub fn get(&self, key: &str) -> Option<String> {
        // Abstracted access; real implementation injected at boundary.
        None
    }
}
