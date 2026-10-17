use vstd::prelude::*;

#[path = "../../crates/titania-core/src/proof_id/kernel.rs"]
mod production;

verus! {

pub open spec fn spec_khi_body_byte(b: int) -> bool {
    (65 <= b && b <= 90) || (97 <= b && b <= 122) || (48 <= b && b <= 57) || b == 95
}

pub assume_specification[production::is_khi_byte](b: u8) -> (ok: bool)
    ensures
        ok == spec_khi_body_byte(b as int),
;

fn khi_accepts_capital_a() -> (ok: bool)
    ensures
        ok == true,
{
    production::is_khi_byte(65)
}

fn khi_rejects_nul() -> (ok: bool)
    ensures
        ok == false,
{
    production::is_khi_byte(0)
}

proof fn khi_body_specs_are_non_vacuous()
    ensures
        spec_khi_body_byte(65)
            && spec_khi_body_byte(90)
            && spec_khi_body_byte(97)
            && spec_khi_body_byte(122)
            && spec_khi_body_byte(48)
            && spec_khi_body_byte(57)
            && spec_khi_body_byte(95),
        !spec_khi_body_byte(0)
            && !spec_khi_body_byte(47)
            && !spec_khi_body_byte(58)
            && !spec_khi_body_byte(64)
            && !spec_khi_body_byte(91)
            && !spec_khi_body_byte(96)
            && !spec_khi_body_byte(123),
{
}

}

fn main() {}
