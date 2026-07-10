// Fixture: triggers BYPASS_GENERATED_INCLUDE.

fn generated_module() {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
