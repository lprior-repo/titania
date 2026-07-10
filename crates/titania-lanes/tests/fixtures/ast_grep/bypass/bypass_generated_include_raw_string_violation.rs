// Fixture: triggers BYPASS_GENERATED_INCLUDE via raw string literals.
// The macro shape `include!(concat!(env!(...), ...))` must be detected
// even when every literal is written as a raw string (`r#"..."#`).
// Tree-sitter parses `r#"OUT_DIR"#` as `raw_string_literal`, not
// `string_literal`, so a kind-only check against `string_literal` lets
// this variant bypass the rule. The detector must decode the inner
// content of both regular and raw string literals and still recognise
// the exact OUT_DIR identifier plus a non-empty path.

fn generated_module() {
    include!(concat!(env!(r#"OUT_DIR"#), r#"/generated.rs"#));
}