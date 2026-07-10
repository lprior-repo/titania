// Fixture: a line comment containing the include!(concat!(env!("OUT_DIR"), ...))
// pattern. The BYPASS_GENERATED_INCLUDE rule must NOT fire — comments are not
// code, and the ast-grep detector matches only AST `macro_invocation` nodes.

fn not_generated() {
    // include!(concat!(env!("OUT_DIR"), "/generated.rs"));
    let _ = "harmless";
}
