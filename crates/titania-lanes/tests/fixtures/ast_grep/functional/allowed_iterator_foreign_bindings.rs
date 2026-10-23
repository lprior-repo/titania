use helpers::iter;
use helpers::repeat as rep;

pub fn via_foreign_module() -> Vec<u8> {
    iter::repeat(7u8).take(2).collect()
}

pub fn via_foreign_alias() -> Vec<u8> {
    rep(7u8).take(2).collect()
}
