// Fixture: a raw string literal containing the include!(concat!(env!("OUT_DIR"), ...))
// pattern. The BYPASS_GENERATED_INCLUDE rule must NOT fire — raw string content
// is not a macro invocation.

fn documentation_example() {
    let example = r#"include!(concat!(env!("OUT_DIR"), "/generated.rs"));"#;
    let _ = example;
}
