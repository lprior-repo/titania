// Fixture: triggers FUNC_RECURSION_DIRECT
// Function calls itself by name from its own body.

pub fn factorial(n: u32) -> u32 {
    if n == 0 {
        1
    } else {
        n.wrapping_mul(factorial(n.saturating_sub(1)))
    }
}

pub fn traverse(depth: u32) -> u32 {
    if depth == 0 {
        0
    } else {
        traverse(depth.saturating_sub(1)).saturating_add(1)
    }
}
