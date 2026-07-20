//! Pure-core parsers for cargo-mutants `outcomes.json` and
//! `mutants.json` artifacts.
//!
//! cargo-mutants 27.0.0 writes a per-package run directory
//! (`mutants.out/`) carrying two top-level files:
//!
//! - `outcomes.json` — one entry per scenario (Baseline + every
//!   mutant), with a `summary` field that names the outcome class
//!   (`Success`, `MissedMutant`, `Unviable`, `Timeout`, `Failure`).
//! - `mutants.json` — a flat array of per-mutant records carrying the
//!   source span, package, file, genre, replacement, and display name.
//!
//! Both parsers are pure-core `&str` boundaries. They declare only the
//! fields v1.5 consumes, so serde's permissive default tolerates unknown
//! top-level keys added by later cargo-mutants versions.

mod outcomes;
mod records;
mod wire;

pub use outcomes::{
    MUTANTS_OUTCOMES_MAX_ENTRIES, MutantOutcomeEntry, MutantScenarioData, MutantsOutcomes,
    OutcomeScenario, OutcomeSummary,
};
pub use records::{
    MUTANTS_RECORDS_MAX_ENTRIES, MutantRecord, MutantsRecords, RawFunction, relative_mutant_path,
};
pub use wire::{RawPoint, RawSpan};
