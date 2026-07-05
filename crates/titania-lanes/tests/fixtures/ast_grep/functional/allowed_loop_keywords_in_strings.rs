// Fixture: allowed — loop keywords appear only in line comments.
//
// Loop keywords in comments must not trigger the functional loop detectors.

/// The loop body uses an iterator pipeline, not a bare loop block.
pub fn process_all(items: &[i32]) -> Vec<i32> {
    // for x in items { do_work(x) }
    // while more { take_next() }
    // loop { keep_going() }
    items.iter().map(|x| x * 2).collect()
}

/// String values describe patterns without using loop syntax.
pub fn describe_loops() -> String {
    let example = "iterate over items and transform each one";
    format!("Example: {example}")
}
