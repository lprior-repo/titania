
#![expect(clippy::needless_return)]

pub fn returns_val(x: i32) -> i32 {
    return x;
}
