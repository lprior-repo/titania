//! Profile-based lane selection.
//!
//! Profiles define which proof/test lanes to run:
//! fast → standard → deep → proof → all (monotonic inclusion).

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Fast,
    Standard,
    Deep,
    ProofOnly,
    All,
}

impl Profile {
    pub fn lanes(&self) -> &'static [&'static str] {
        match self {
            Profile::Fast => &["test", "clippy"],
            Profile::Standard => &["test", "clippy", "nextest"],
            Profile::Deep => &["test", "clippy", "nextest", "kani", "miri", "loom"],
            Profile::ProofOnly => &[
                "test", "clippy", "nextest", "kani", "miri", "loom", "verus", "tla", "flux",
            ],
            Profile::All => &[
                "test", "clippy", "nextest", "kani", "miri", "loom", "fuzz", "mutants", "coverage",
                "verus", "tla", "flux",
            ],
        }
    }
}

pub fn parse_profile(value: &str) -> Option<Profile> {
    match value {
        "fast" => Some(Profile::Fast),
        "standard" => Some(Profile::Standard),
        "deep" => Some(Profile::Deep),
        "proof" => Some(Profile::ProofOnly),
        "all" => Some(Profile::All),
        _ => None,
    }
}

pub fn is_monotonic() -> bool {
    let profiles = [
        Profile::Fast,
        Profile::Standard,
        Profile::Deep,
        Profile::ProofOnly,
        Profile::All,
    ];

    for window in profiles.windows(2) {
        let Some(first) = window.first() else {
            continue;
        };
        let Some(second) = window.get(1) else {
            continue;
        };
        let a: HashSet<_> = first.lanes().iter().copied().collect();
        let b: HashSet<_> = second.lanes().iter().copied().collect();
        if !a.is_subset(&b) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_monotonicity() {
        assert!(
            is_monotonic(),
            "Profile lane sets must be monotonically increasing"
        );
    }

    #[test]
    fn test_fast_lanes() {
        let lanes = Profile::Fast.lanes();
        assert!(lanes.contains(&"test"));
        assert!(lanes.contains(&"clippy"));
        assert_eq!(lanes.len(), 2);
    }

    #[test]
    fn test_all_lanes() {
        let lanes = Profile::All.lanes();
        assert!(lanes.contains(&"test"));
        assert!(lanes.contains(&"kani"));
        assert!(lanes.contains(&"verus"));
    }

    #[test]
    fn test_parse_profile() {
        assert!(parse_profile("fast").is_some());
        assert!(parse_profile("standard").is_some());
        assert!(parse_profile("deep").is_some());
        assert!(parse_profile("proof").is_some());
        assert!(parse_profile("all").is_some());
        assert!(parse_profile("invalid").is_none());
    }
}
