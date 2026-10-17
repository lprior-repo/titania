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
pub use obligations::{ObligationError, ObligationKind, ObligationSet};
pub use receipt_builder::{ReceiptBuilderError, build_quality_receipt, compute_evidence_digest};
pub use report_assembly::{ReportAssemblyError, assemble_report, assemble_report_with_obligations};

pub mod artifact_reader;
pub mod obligations;
pub mod receipt_builder;
pub mod report_assembly;
