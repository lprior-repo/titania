#![cfg(loom)]

use std::path::{Path, PathBuf};

use loom::{
    sync::{Arc, Mutex},
    thread,
};
use tempfile::TempDir;
use titania_core::{MutantBaselineEntry, MutantId, MutantOperator, MutantsBaseline};

const WRITER_ITERATIONS: usize = 5;

const READER_ITERATIONS: usize = 5;

const EXPECTED_ENTRY_COUNT: usize = 2;

const MODEL_MAX_THREADS: usize = 3;

const MODEL_PREEMPTION_BOUND: usize = 2;

#[test]
fn atomic_baseline_write_under_concurrent_read() {
    let tmp = TempDir::new().expect("tempdir");
    let baseline_path: PathBuf = tmp.path().join(".titania").join("baseline.json");
    write_baseline_atomically(&baseline_path);

    let mut builder = loom::model::Builder::new();
    builder.max_threads = MODEL_MAX_THREADS;
    builder.preemption_bound = Some(MODEL_PREEMPTION_BOUND);

    builder.check(move || {
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
        let reader_result: Arc<Mutex<Option<MutantsBaseline>>> = shared_result.clone();
        let reader_observer: Arc<Mutex<Option<MutantsBaseline>>> = reader_result.clone();
        let reader_handle = thread::spawn(move || {
            for _ in 0..READER_ITERATIONS {
                let path: PathBuf = {
                    let guard = reader_path.lock().expect("path mutex poisoned");
                    guard.clone()
                };
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

fn read_complete_baseline(path: &Path) -> Result<MutantsBaseline, String> {
    let label: String = path.display().to_string();
    let contents: String = std::fs::read_to_string(path)
        .map_err(|error| format!("baseline file unreadable at `{label}`: {error}"))?;
    MutantsBaseline::parse_str(&contents, &label)
        .map_err(|error| format!("baseline file at `{label}` failed to parse: {error}"))
}

fn write_baseline_atomically(path: &Path) {
    let payload: String = baseline_json_blob();
    let parent: &Path = path.parent().expect("baseline path has a parent directory");
    std::fs::create_dir_all(parent).expect("artifact parent directory must be creatable");
    let stem: &str = path.file_name().and_then(|name| name.to_str()).unwrap_or("baseline.json");
    let temp: PathBuf = parent.join(format!(".titania-baseline-{stem}.tmp"));
    std::fs::write(&temp, payload.as_bytes()).expect("temp write must succeed");
    std::fs::rename(&temp, path).expect("atomic rename must succeed");
}

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
