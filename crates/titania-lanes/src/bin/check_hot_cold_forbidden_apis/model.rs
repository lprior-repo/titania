use std::convert::TryFrom;

pub const HOT_CRATES: &[&str] = &["titania-core", "titania-lanes"];

pub const COLD_MARKERS: &[&str] = &[
    "diagnostic",
    "diagnostics",
    "fixture",
    "fixtures",
    "harness",
    "kani",
    "loom",
    "proof",
    "property",
    "proptest",
    "proptests",
    "support",
    "test_util",
    "tests",
    "verification",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FindingData {
    pub rel_path: String,
    pub line_no: usize,
    pub class_id: &'static str,
    pub text: String,
}

impl FindingData {
    pub fn line_no_as_u32(&self) -> u32 {
        u32::try_from(self.line_no).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceRole {
    HotProduction,
    LaneBinary,
    Test,
    ColdSupport,
}
