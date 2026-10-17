
fn generated_module() {
    include!(concat!(env!(r#"OUT_DIR"#), r#"/generated.rs"#));
}