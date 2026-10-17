
fn load_generated_module() {
    include!(concat!(env!("OUT_DIR"), "/generated_module.rs"));
}
