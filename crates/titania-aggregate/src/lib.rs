//! v1 lane-artifact reader for Titania.
//!
//! Reads expected lane-output JSON files in Edit/Prepush/Release order from a
//! target project's `.titania/out/` directory.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

pub use artifact_reader::{ReaderError, ReaderResult, read_lane_artifacts};
pub use receipt_builder::{ReceiptBuilderError, build_quality_receipt, compute_evidence_digest};
pub use report_assembly::{ReportAssemblyError, assemble_report};

pub mod artifact_reader;
pub mod receipt_builder;
pub mod report_assembly;
