// Fixture: allowed — loop keywords only inside raw string literals.

pub fn raw_loop_docs() -> &'static str {
    r#"let quoted = "value"; for item in items { do_work(item) }
let again = "q"; while ready { tick() }
loop { retry() }"#
}
