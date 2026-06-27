//! Lane runners: each wraps an external tool, parses output, maps to findings.

pub mod check;
pub mod clippy;
pub mod fmt;
pub mod panic_scan;
pub mod process;
pub mod registry;

pub use check::CheckLane;
pub use clippy::ClippyLane;
pub use fmt::FmtLane;
pub use panic_scan::PanicAssertScanLane;
pub use registry::{LaneRegistry, LaneRunner};
