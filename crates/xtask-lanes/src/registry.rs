//! Lane registry: maps Lane enums to runner implementations.

use xtask_core::{Lane, LaneOutcome};

/// A trait for running a single quality lane.
pub trait LaneRunner {
    /// Which lane this runner handles.
    fn lane(&self) -> Lane;

    /// Run the lane and return its outcome.
    fn run(&self) -> LaneOutcome;
}

/// Registry of all available lane runners.
pub struct LaneRegistry {
    runners: Vec<Box<dyn LaneRunner>>,
}

impl LaneRegistry {
    /// Build a registry with the given runners.
    #[must_use]
    pub fn new(runners: Vec<Box<dyn LaneRunner>>) -> Self {
        Self { runners }
    }

    /// Find the runner for a specific lane.
    #[must_use]
    pub fn get(&self, lane: Lane) -> Option<&dyn LaneRunner> {
        self.runners
            .iter()
            .map(Box::as_ref)
            .find(|r| r.lane() == lane)
    }
}
