// Fixture: triggers FUNC_NESTING_DEPTH
// Function body nests blocks deeper than the allowed depth (max 2).

pub fn classify(values: &[i32]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for v in values {
        if *v > 0 {
            if *v % 2 == 0 {
                if *v > 100 {
                    out.push("big-even-positive");
                } else {
                    out.push("small-even-positive");
                }
            } else if *v > 100 {
                out.push("big-odd-positive");
            } else {
                out.push("small-odd-positive");
            }
        } else if *v < 0 {
            out.push("negative");
        } else {
            out.push("zero");
        }
    }
    out
}
