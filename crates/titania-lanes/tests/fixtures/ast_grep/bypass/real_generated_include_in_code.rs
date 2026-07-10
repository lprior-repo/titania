// Fixture: a real generated include in code. The BYPASS_GENERATED_INCLUDE rule
// MUST fire — this is the positive control proving the detector still detects
// the structural pattern when it appears in executable code.

fn load_generated_module() {
    include!(concat!(env!("OUT_DIR"), "/generated_module.rs"));
}
