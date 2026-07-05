// Fixture: allowed — loop keywords only inside string literals.
//
// String literals containing `for`, `while`, `loop` must NOT trigger
// FUNC_LOOPS_* findings.

pub fn format_loop_doc() -> String {
    let for_example = "for item in items { process(item) }";
    let while_example = "while counter < max { retry() }";
    let loop_example = "loop { forever(); break; }";
    format!("{for_example}\n{while_example}\n{loop_example}")
}

pub fn describe_control_flow() -> &'static str {
    "Use `for` in Rust to iterate, `while` for condition-based loops, \
     and `loop { ... }` for explicit infinite loops"
}
