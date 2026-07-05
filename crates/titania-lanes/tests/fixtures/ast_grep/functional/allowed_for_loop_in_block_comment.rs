// Fixture: allowed — for-loop text only in block-comment bodies.
//
// Lines inside `/* ... */` that do NOT start with `//`, `/*`, or `*`
// are currently NOT filtered by code_line_contains. This file must
// stay clean once the lane handles block-comment body lines.

/// Documentation comments are not block comments — they are OK.
/*
for x in items {
    process(x);
    while more {
        continue;
    }
    loop {
        break;
    }
}
*/

pub fn process_items(items: &[i32]) -> i32 {
    items.iter().sum()
}
