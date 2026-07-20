//! Loom concurrency model — atomic baseline WRITE under concurrent READ.
//!
//! **Compile-only:** loom permutation tests are intentionally slow; this file
//! is gated on `#[cfg(loom)]` and verified with
//! `RUSTFLAGS="--cfg loom" cargo test --release -p titania-lanes --test v15_atomic_baseline`
//! under bounded `LOOM_MAX_PREEMPTIONS` so the interleaving exploration stays
//! tractable.
//!
//! The contract under test (v1.5 contract clause 8): a baseline JSON file is
//! produced via atomic temp-file-then-rename so concurrent readers must
//! always observe either the previous baseline or the new one — **never a
//! half-written artefact**. This module exercises that contract end-to-end:
//! one writer thread repeatedly stamps the baseline through the production
//! atomic-rename pattern (`write` to a `.tmp`, then `rename` over the final
//! path) while one reader thread repeatedly loads the file via the pure
//! `MutantsBaseline::parse_str` API. Loom permutes every interleaving of the
//! spawn / lock / join boundaries to assert the invariant.
//!
//! Reader-side invariant: every reader observation must produce a fully
//! typed `MutantsBaseline` whose `entries` vector carries exactly the two
//! entries the writer stamps. **A missing file, an unreadable file, or a
//! parse error means the reader saw a half-written artefact — that is a
//! contract violation and must fail the permutation, not be silently
//! tolerated.**
//!
//! `PathBuf` is `!Sync` under `cfg(loom)`, and `MutantsBaseline` carries
//! non-`Sync` `String` payloads, so both are wrapped in `loom::sync::Mutex`
//! and shared through `loom::sync::Arc`; this lets the writer and reader
//! hand off `PathBuf` and observation slots without violating loom's
//! cell-tracked `Sync` requirements.

#![cfg(loom)]

use std::path::{Path, PathBuf};

use loom::{
    sync::{Arc, Mutex},
    thread,
};
use tempfile::TempDir;
use titania_core::{MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline};

/// Number of full atomic-rename round-trips the writer performs inside one
/// `loom::model` permutation.
const WRITER_ITERATIONS: usize = 5;

/// Number of `MutantsBaseline::parse_str` calls the reader performs inside
/// one `loom::model` permutation. Pinned equal to [`WRITER_ITERATIONS`] so
/// the model always has at least one reader-side observation per writer run.
const READER_ITERATIONS: usize = 5;

/// Complete-baseline marker: the production payload stamps exactly two
/// entries through [`baseline_json_blob`], so any successful load that
/// observes a fully-written artefact must report both — never one, never
/// zero, never a partial third.
const EXPECTED_ENTRY_COUNT: usize = 2;

/// Explicit thread bound for the loom permutation: main thread + writer +
/// reader. Keeping `max_threads` small is required by the loom
/// `MAX_THREADS` ceiling and shrinks the explored state space.
const MODEL_MAX_THREADS: usize = 3;

/// Explicit preemption bound for the loom permutation. Each writer
/// iteration holds the path mutex only long enough to clone the
/// `PathBuf`; the heavy I/O happens lock-free. A small preemption bound
/// therefore catches the meaningful publish/observe interleavings without
/// exploding the permutation count.
const MODEL_PREEMPTION_BOUND: usize = 2;

/// Bounded loom model: one writer + one reader. The writer stamps the
/// baseline through the production atomic-rename pattern while the reader
/// reloads the file under every interleaving loom explores, asserting that
/// no reader observation ever reports a missing file, an unreadable file,
/// or a parse failure — all three would mean a half-written artefact.
#[test]
fn atomic_baseline_write_under_concurrent_read() {
    // Setup outside the model: create the workspace, pre-write the initial
    // baseline so every reader observation has a complete document to find.
    let tmp = TempDir::new().expect("tempdir");
    let baseline_path: PathBuf = tmp.path().join(".titania").join("baseline.json");
    write_baseline_atomically(&baseline_path);

    // Configure bounded loom exploration explicitly so the model cannot
    // silently grow threads or scheduler depth.
    let mut builder = loom::model::Builder::new();
    builder.max_threads = MODEL_MAX_THREADS;
    builder.preemption_bound = Some(MODEL_PREEMPTION_BOUND);

    builder.check(move || {
        // All loom-aware synchronisation happens inside this closure:
        // the path is shared through `Arc<Mutex<PathBuf>>` and the
        // observation slot through `Arc<Mutex<Option<MutantsBaseline>>>`.
        let shared_path: Arc<Mutex<PathBuf>> = Arc::new(Mutex::new(baseline_path.clone()));
        let shared_result: Arc<Mutex<Option<MutantsBaseline>>> =
            Arc::new(Mutex::new(None::<MutantsBaseline>));

        let writer_path: Arc<Mutex<PathBuf>> = shared_path.clone();
        let writer_handle = thread::spawn(move || {
            for _ in 0..WRITER_ITERATIONS {
                let path: PathBuf = {
                    let guard = writer_path.lock().expect("path mutex poisoned");
                    guard.clone()
                };
                write_baseline_atomically(&path);
            }
        });

        let reader_path: Arc<Mutex<PathBuf>> = shared_path.clone();
        // Clone the result slot before moving it into the reader closure so
        // the parent thread can still observe the last successful parse after
        // the reader joins.
        let reader_result: Arc<Mutex<Option<MutantsBaseline>>> = shared_result.clone();
        let reader_observer: Arc<Mutex<Option<MutantsBaseline>>> = reader_result.clone();
        let reader_handle = thread::spawn(move || {
            for _ in 0..READER_ITERATIONS {
                let path: PathBuf = {
                    let guard = reader_path.lock().expect("path mutex poisoned");
                    guard.clone()
                };
                // Reader must observe a complete baseline; parse errors and
                // half-written content violate the atomic-rename contract
                // and must fail the permutation, not be ignored.
                let loaded = read_complete_baseline(&path).expect(
                    "reader observed a missing, unreadable, or malformed baseline; \
                     atomic-rename contract requires readers see only complete old/new documents",
                );
                assert_eq!(
                    loaded.entries().len(),
                    EXPECTED_ENTRY_COUNT,
                    "successful parse must report a complete baseline with the expected entry count",
                );
                let mut slot = reader_result.lock().expect("result mutex poisoned");
                *slot = Some(loaded);
            }
        });

        writer_handle.join().expect("writer thread must not panic");
        reader_handle.join().expect("reader thread must not panic");

        let recorded: Option<MutantsBaseline> = {
            let guard = reader_observer.lock().expect("result mutex poisoned");
            guard.clone()
        };
        let last: &MutantsBaseline = recorded
            .as_ref()
            .expect("reader must record at least one successful load");
        assert_eq!(
            last.entries().len(),
            EXPECTED_ENTRY_COUNT,
            "every recorded observation must be a complete baseline with the expected entry count",
        );
    });
}

/// Read baseline bytes and run them through the pure
/// `MutantsBaseline::parse_str` API. A reader under the atomic-rename
/// contract must always observe a complete baseline — any read or parse
/// failure means the reader saw a half-written artefact or a missing
/// file, which violates the contract.
///
/// # Errors
/// - `Err` when `path` cannot be read (missing file, permission denied,
///   non-UTF-8 bytes, etc.). The error string carries the underlying
///   `std::io::Error` description for triage.
/// - `Err` when `MutantsBaseline::parse_str` rejects the contents (JSON
///   parse failure, unsupported schema version, malformed entry shape).
///   The error string carries the typed `MutantsBaselineError` description.
fn read_complete_baseline(path: &Path) -> Result<MutantsBaseline, String> {
    let label: String = path.display().to_string();
    let contents: String = std::fs::read_to_string(path)
        .map_err(|error| format!("baseline file unreadable at `{label}`: {error}"))?;
    MutantsBaseline::parse_str(&contents, &label)
        .map_err(|error| format!("baseline file at `{label}` failed to parse: {error}"))
}

/// Write the mutant baseline payload to `path` using the production atomic
/// temp-file-then-rename contract:
///
/// 1. Stage the JSON blob in a sibling `.tmp` file.
/// 2. Atomically `rename` the `.tmp` file over the final path.
///
/// Under every loom permutation, a reader of `path` therefore observes
/// either the prior contents or the just-renamed contents — never a
/// half-written artefact.
fn write_baseline_atomically(path: &Path) {
    let payload: String = baseline_json_blob();
    let parent: &Path = path.parent().expect("baseline path has a parent directory");
    std::fs::create_dir_all(parent).expect("artifact parent directory must be creatable");
    let stem: &str = path.file_name().and_then(|name| name.to_str()).unwrap_or("baseline.json");
    let temp: PathBuf = parent.join(format!(".titania-baseline-{stem}.tmp"));
    std::fs::write(&temp, payload.as_bytes()).expect("temp write must succeed");
    std::fs::rename(&temp, path).expect("atomic rename must succeed");
}

/// Build the complete mutants-baseline JSON the writer stamps to disk and
/// the reader must observe under every successful load. The two
/// `MutantBaselineEntry` rows are stable and unique per test invocation so
/// any observed-baseline mismatch is unambiguous.
fn baseline_json_blob() -> String {
    let entries = vec![
        MutantBaselineEntry {
            mutation_id: MutantId::new("pkg-a", "src/a.rs", 1, 1, MutantOperator::EqualReplace)
                .unwrap_or_else(|error| {
                    panic!(
                        "test fixture id `pkg-a::src/a.rs:1:1:equal_replace` must construct: {error}"
                    )
                }),
            accepted_by_rule: "mutant-accept/owner-a/reason-a/never".into(),
            reason: "loom concurrency under check".into(),
            expires_on_unix: None,
        },
        MutantBaselineEntry {
            mutation_id: MutantId::new("pkg-b", "src/b.rs", 1, 1, MutantOperator::EqualReplace)
                .unwrap_or_else(|error| {
                    panic!(
                        "test fixture id `pkg-b::src/b.rs:1:1:equal_replace` must construct: {error}"
                    )
                }),
            accepted_by_rule: "mutant-accept/owner-b/reason-b/never".into(),
            reason: "loom concurrency under check".into(),
            expires_on_unix: None,
        },
    ];
    let baseline: MutantsBaseline = MutantsBaseline::from_bypasses(entries);
    serde_json::to_string_pretty(&baseline).expect("baseline serialises")
}
