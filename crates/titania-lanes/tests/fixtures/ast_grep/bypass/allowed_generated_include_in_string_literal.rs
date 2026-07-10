// Fixture: a string literal containing the include!(concat!(env!("OUT_DIR"), ...))
// pattern. The BYPASS_GENERATED_INCLUDE rule must NOT fire — string content is
// not a macro invocation.

fn documentation_example() {
    let example = "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));";
    let _ = example;
}
