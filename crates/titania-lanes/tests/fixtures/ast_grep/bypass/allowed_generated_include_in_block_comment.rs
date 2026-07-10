// Fixture: a block comment containing the include!(concat!(env!("OUT_DIR"), ...))
// pattern. The BYPASS_GENERATED_INCLUDE rule must NOT fire — block comments are
// not code.

fn not_generated() {
    /* include!(concat!(env!("OUT_DIR"), "/generated.rs")); */
    let _ = "harmless";
}
