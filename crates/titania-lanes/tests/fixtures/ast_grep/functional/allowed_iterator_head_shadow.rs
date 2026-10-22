use std::iter;

pub fn via_other_head() -> Vec<u8> {
    other::iter::repeat(7u8).take(2).collect()
}

pub fn bounded_member_of_bound_module() -> Vec<u8> {
    iter::empty().collect()
}
