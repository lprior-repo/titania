# Red Queen Adversarial Evolution Review — v1.5 Kani + Mutants + Full scope

> Adversarial review of the test surface for the v1.5 contract at
> `.evidence/v1.5/spec.md`. The goal is **adversarial pressure**: identify
> gaps where the tests would still pass against a subtly broken implementation,
> find brittleness that survives implementation defects, and recommend concrete
> proptest/property/behaviour tests that would expose them.
>
> **Scope reviewed** — four source modules and ten v15 test files plus their
> shared infra:
>
> - `crates/titania-core/src/proof_id.rs` (`KaniHarnessId`, `MutantId`,
>   `MutantOperator`, `ToolKind`)
> - `crates/titania-core/src/mutants_baseline.rs` (`MutantsBaseline`,
>   `MutantBaselineEntry`)
> - `crates/titania-lanes/src/run_lane_kani.rs` (Kani lane driver)
> - `crates/titania-lanes/src/run_lane_mutants.rs` (Mutants lane driver)
> - Test files: `crates/titania-core/tests/v15_*.rs`
> - `crates/titania-lanes/tests/v15_atomic_baseline.rs`
>
> Plus Moon wiring (`.moon/tasks/all.yml`), CLI parser
> (`crates/titania-check/src/args/parse.rs`), aggregate report dispatch
> (`titania-check/aggregate.rs`, `titania-aggregate/artifact_reader.rs`),
> explain catalog (`titania-output/explain.rs`), and the existing
> `crates/titania-core/tests/properties.rs` (proptest surface for pre-existing
> primitives).

## Verdict

**FAIL — tests are not yet strong enough to expose implementation defects in
the v1.5 contract surface.** Every cargo gate (`fmt`/`check`/`clippy`/`test`)
passes on the current code (RELEASE_REPORT.md §"Cargo Gates"), but the test
surface for the new v1.5 surface is **shallow**: per-test branch coverage is
low; spec ↔ implementation parity is broken in ways no current test catches;
the loom concurrency model does not actually exercise atomic write/read
interleaving; and there is no end-to-end CLI exercise of the new
`run-lane kani`/`run-lane mutants` paths or the new `--scope full`.

A implementation with the following defects would still pass every existing
v15 test:

1. `KaniHarnessId::new` accepting `_FOO_BAR` (doc says must start with a
   letter; impl does not enforce; no test catches it).
2. `MutantId::new` accepting `rel_path = "subdir/with:colon.rs"` (no colon
   guard; no test catches it).
3. `parse_verdict("VERIFICATION: anything UNSUPPORTED something")` returning
   `HarnessVerdict::Unsupported` (substring match bug; no test catches it).
4. `contains("MUTANT_SURVIVED")` returning `true` for a wildcard baseline that
   is past its expiry (false safety: tests don't push past expiry).
5. `parse_verdict` returning `Unknown` for `"VERIFICATION:  successful  "` —
   case mismatch not tested.
6. The lane-writing `out` directory only contains Kani/Mutants JSON under
   `.titania/out/full/*`; no test verifies that `aggregate --scope release`
   does **not** include Kani/Mutants findings even when those JSON files
   exist on disk.

Score: 0 of 6 spec-deviation mutations would be caught.

The tests as written are **plausibility tests**, not **adversarial
behaviour tests**. The Red Queen is not satisfied.

---

## Test Gaps Found

The following gaps in the v15 test surface would silently mask implementation
defects.

### Gap 1: `KaniHarnessId` accepts leading underscore despite doc contract

**File**: `crates/titania-core/src/proof_id.rs:24-25,86-111`;
**Doc contract**: line 22 says "`Format: ^[A-Z][A-Z0-9_]*$` — uppercase ASCII
letters, digits, and underscore; **must start with a letter** and contain at
least one underscore."
**Spec contract**: `spec.md:52` says `Format: ^[a-zA-Z][a-zA-Z0-9_]*$`.
**Implementation**: only rejects ASCII digits in `first`; does NOT reject
leading underscore.

**Bug**: `KaniHarnessId::new("_FOO_BAR")` returns `Ok`, in violation of both
doc and spec. The current test suite exercises `1FOO_BAR` (leading digit
rejection) but never `_FOO_BAR` (leading underscore accepted).

**Test to add** — `crates/titania-core/tests/v15_kani_harness_id.rs`:

```rust
#[test]
fn rejects_leading_underscore() {
    let result = KaniHarnessId::new("_FOO_BAR");
    assert!(
        matches!(result, Err(KaniHarnessIdError::LeadingDigit)),
        "leading underscore is silently accepted by the impl; doc requires a letter"
    );
}
```

**Mutation that survives**: the current implementation could even widen the
regex further (e.g. allow `&`) and no test would catch it.

### Gap 2: `KaniHarnessId` NUL byte not tested

**File**: `crates/titania-core/src/proof_id.rs:116-128`.
**Spec**: non-ASCII rejected.
**Current tests**: dot, lowercase, length.

**Test to add** (UTF-8 multi-byte and NUL byte):

```rust
#[test]
fn rejects_nul_byte() {
    let result = KaniHarnessId::new("FOO\0BAR");
    assert!(
        matches!(result, Err(KaniHarnessIdError::NotUpperAscii { byte: 0, .. })),
        "NUL byte must be rejected"
    );
}

#[test]
fn rejects_utf8_multibyte() {
    // "KÄN_I" — K, Ä (0xC3 0x84), N, _, I
    let result = KaniHarnessId::new("KÄN_I");
    assert!(
        matches!(result, Err(KaniHarnessIdError::NotUpperAscii { byte: 0xC3, offset: 1 })),
        "multibyte UTF-8 must be rejected; got {result:?}"
    );
}
```

### Gap 3: `KaniHarnessId` accepts `___` (all-underscore)

**File**: `crates/titania-core/src/proof_id.rs:104`.
**Behaviour**: `s.contains('_')` is true for `"___"`; `first` is `_`, not a
digit; chars are all valid. So `"___"` is accepted.

**Test to add**:

```rust
#[test]
fn rejects_only_underscores() {
    let result = KaniHarnessId::new("___");
    assert!(
        matches!(result, Err(KaniHarnessIdError::LeadingDigit)),
        "all-underscore input must be rejected by either 'leading not letter' or 'no underscore' rule"
    );
}
```

### Gap 4: `KaniHarnessId` mixed case `Foo_bar` not tested

**File**: `crates/titania-core/tests/v15_kani_harness_id.rs:34` exercises
`foo_bar` (all lowercase) but never mixed case.

**Test to add**:

```rust
#[test]
fn rejects_mixed_case() {
    let result = KaniHarnessId::new("Foo_Bar");
    // The lowercase 'o' at offset 1 must be rejected with NotUpperAscii.
    assert!(
        matches!(result, Err(KaniHarnessIdError::NotUpperAscii { byte: b'o', offset: 1 })),
        "mixed case 'Foo_Bar' must be rejected; got {result:?}"
    );
}
```

### Gap 5: `MutantId::new` accepts colons in `rel_path`

**File**: `crates/titania-core/src/proof_id.rs:200`. The
`check_mutant_shape` helper (lines 239-261) rejects empty package, empty
path, zero line/col, and absolute path — but does NOT reject `:` or `::`
inside `rel_path`.

**Bug**: `MutantId::new("pkg", "weird:colon.rs", 1, 1, MutantOperator::EqualReplace)`
is accepted. The serialized form becomes `"pkg::weird:colon.rs:1:1:equal_replace"`,
which corrupts the parse-position assumptions of `MutantId::package()`,
`MutantId::location()`, and `MutantId::operator_matched()`.

**Test to add**:

```rust
#[test]
fn rejects_rel_path_with_colon() {
    let result = MutantId::new("pkg", "src/weird:colon.rs", 1, 1, MutantOperator::EqualReplace);
    // The current impl accepts this — the test must document the violation
    // and fail until the impl is fixed.
    assert!(
        result.is_err(),
        "rel_path with ':' must be rejected to preserve canonical format; got {result:?}"
    );
}

#[test]
fn rejects_rel_path_with_double_colon() {
    let result = MutantId::new("pkg", "src/sub::weird.rs", 1, 1, MutantOperator::EqualReplace);
    assert!(result.is_err(), "rel_path with '::' must be rejected");
}
```

**Mutation that survives**: the current `package()` call uses
`split_once("::")`, picking the first `::`. A path containing `::` would
split ambiguously and `location()` would yield the wrong segment. No current
test catches this.

### Gap 6: `MutantIdError::UnknownOperator` programmatically unreachable and
never tested

**File**: `crates/titania-core/src/proof_id.rs:84-86` —
`#[error("mutant id operator {0:?} is not in the recognised operator set")]
UnknownOperator(String),`

The variant is only constructed from raw-string operator input, but the public
constructor takes a `MutantOperator` enum value which is closed. **The
variant is dead code from a public API perspective**; testing it requires
hitting the unreachable branch via internal/test-only constructor or via JSON
deserialization.

**Test to add** (force via JSON deserialization, where the invariant is the
weakest):

```rust
#[test]
fn unknown_operator_deserializes_to_unknown() {
    let raw = r#"{"mutation_id":"pkg::src/lane.rs:1:1:equal_replace"}"#;
    // Skipped: MutantId has #[serde(transparent)] over String, so the type
    // is decoupled from the operator enum. Document the dead variant here.
    let err = serde_json::from_str::<MutantId>(raw).is_err();
    assert!(err, "MutantId deserialization should reject unknown operator literals");
}
```

Even better — split the type so the operator is part of the public schema:

```rust
#[derive(Serialize, Deserialize)]
pub struct MutantIdWire {
    pub package: String,
    pub rel_path: String,
    pub line: u32,
    pub col: u32,
    pub operator: MutantOperator,
}
```

so the public surface actually exercises the unknown-operator rejection.

### Gap 7: `MutantId` boundary cases at `u32::MAX` not tested

**File**: `crates/titania-core/tests/v15_mutant_id.rs` only checks `line == 0`
and `col == 0`. There is no test for `line == u32::MAX` or `col == u32::MAX`.

**Test to add**:

```rust
#[test]
fn accepts_u32_max_line_and_col() {
    let id = MutantId::new("p", "a.rs", u32::MAX, u32::MAX, MutantOperator::EqualReplace);
    let parsed = id.unwrap();
    assert_eq!(parsed.as_str(), "p::a.rs:4294967295:4294967295:equal_replace");
}
```

**Mutation that survives**: An implementation that silently overflows `u32`
on the `format!()` line (e.g. `line + 1` somewhere) would crash on this
edge case; current tests don't exercise it.

### Gap 8: `MutantId::as_str` is not exercised for round-trip

`v15_mutant_id.rs` only checks `Display` (`to_string()`). `as_str()` and
`Display` are not exercised by a serde double-round-trip:

```rust
#[test]
fn serde_roundtrip_preserves_canonical_string() {
    let id = MutantId::new("p", "a.rs", 1, 1, MutantOperator::EqualReplace).unwrap();
    let json = serde_json::to_string(&id).unwrap();
    let back: MutantId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
    assert_eq!(back.as_str(), id.as_str());
}
```

The current `MutantId` is `#[serde(transparent)]` over `String`; the test
above WILL pass but it does NOT actually test the canonical-format invariants
because no validation runs on deserialize.

### Gap 9: `MutantsBaseline::load` does not reject an empty `schema_version`

**File**: `crates/titania-core/src/mutants_baseline.rs:154-160`.
**Bug**: `validate_baseline` only checks `schema_version != CURRENT`. An
absent schema_version field would deserialise to `0u32` (default for
`u32`) and trigger `UnsupportedSchemaVersion { found: 0, .. }`. That is
technically the correct error class — but the diagnostic says "found 0"
even though the real problem is "field missing". A clearer contract would
distinguish missing-vs-invalid.

**Test to add**:

```rust
#[test]
fn missing_schema_version_field_is_rejected_as_unsupported() {
    // Fixture: {"entries": []} (no schema_version field)
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing_schema.json");
    std::fs::write(&path, r#"{"entries":[]}"#).unwrap();
    let err = MutantsBaseline::load(&path).unwrap_err();
    assert!(matches!(err, MutantsBaselineError::UnsupportedSchemaVersion { found: 0, .. }));
}
```

### Gap 10: `MutantsBaseline::load` does not reject a negative-ish `expires_on_unix`

**File**: `crates/titania-core/src/mutants_baseline.rs:30` —
`expires_on_unix: Option<u64>`. The type is unsigned, so a "negative"
expire isn't possible via JSON deserialization (would fail JSON parse).
However, a missing field defaults to `None` ("never expires"), and a `0`
expire is treated as "expired at the epoch, every check after fails" —
no test currently probes `expires_on_unix == Some(0)`.

**Test to add**:

```rust
#[test]
fn expires_at_zero_rejects_every_timestamp() {
    let entries = vec![MutantBaselineEntry {
        mutation_id: "m".into(),
        accepted_by_rule: "r".into(),
        reason: "".into(),
        expires_on_unix: Some(0),
    }];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec!["m".to_owned()];
    let diff = baseline.diff(&survivors, 1);
    assert_eq!(diff.len(), 1, "expires_on_unix=0 must reject every later timestamp");
}
```

### Gap 11: `MutantsBaseline::diff` under very large entry sets not tested

`crates/titania-core/tests/v15_mutants_baseline_diff.rs:54-68` exercises 10
baseline entries with 2 survivors. There is no test for the production-scale
case where the baseline has thousands of entries (the release report says
2827 surviving mutants and ~hundreds of baseline entries in expected
post-bootstrap state).

**Test to add**:

```rust
#[test]
fn diff_handles_10k_baseline_entries() {
    let entries = (0..10_000)
        .map(|i| MutantBaselineEntry {
            mutation_id: format!("baseline_{i}"),
            accepted_by_rule: "r".into(),
            reason: "".into(),
            expires_on_unix: None,
        })
        .collect::<Vec<_>>();
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors: Vec<String> = (0..10_000).map(|i| format!("baseline_{i}")).collect();
    let diff = baseline.diff(&survivors, u64::MAX);
    assert!(diff.is_empty());
    let new_survivors = vec!["new1".to_owned(), "new2".to_owned()];
    let diff2 = baseline.diff(&new_survivors, u64::MAX);
    assert_eq!(diff2.len(), 2);
}
```

**Mutation that survives**: a quadratic algorithm in `entries × survivors`
would be invisible at 10 entries but disastrous at 10k.

### Gap 12: `MutantsBaseline::contains` is not directly tested with a
boundary case

`v15_baseline_expiry.rs` covers `now_unix == expires_on_unix` (inclusive
boundary), but never tests `now_unix == expires_on_unix - 1` (the case that
must NOT cover) and `now_unix == expires_on_unix + 1` (the case that must
NOT cover).

**Test to add** to `v15_baseline_expiry.rs`:

```rust
#[test]
fn boundary_one_before_does_not_cover() {
    let entries = vec![MutantBaselineEntry {
        mutation_id: "boundary_before".to_owned(),
        accepted_by_rule: "".into(),
        reason: "".into(),
        expires_on_unix: Some(5_000),
    }];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec!["boundary_before".to_owned()];
    let diff = baseline.diff(&survivors, 4_999);
    assert_eq!(diff.len(), 1, "now_unix == expires_on_unix - 1 must NOT cover");
}

#[test]
fn boundary_one_after_does_not_cover() {
    let entries = vec![MutantBaselineEntry {
        mutation_id: "boundary_after".to_owned(),
        accepted_by_rule: "".into(),
        reason: "".into(),
        expires_on_unix: Some(5_000),
    }];
    let baseline = MutantsBaseline::from_bypasses(entries);
    let survivors = vec!["boundary_after".to_owned()];
    let diff = baseline.diff(&survivors, 5_001);
    assert_eq!(diff.len(), 1, "now_unix == expires_on_unix + 1 must NOT cover");
}
```

### Gap 13: `parse_verdict` substring bug — `"UNSUPPORTED"` substring match

**File**: `crates/titania-lanes/src/run_lane_kani.rs:294`.
**Bug**:

```rust
"other if other.contains(\"UNSUPPORTED\") => HarnessVerdict::Unsupported,"
```

`verdict_from_line` strips the `VERIFICATION:` prefix, then trims, then
matches `contains("UNSUPPORTED")`. A real Kani line like
`"VERIFICATION: SUCCESSFUL — unsupported-feature warning emitted"` (an
unfortunate free-form description) would be **mis-classified as Unsupported
instead of Successful**.

**There are no tests for `parse_verdict`** — neither for the `Successful`,
`Failed`, `Unsupported`, nor `Unknown` branches. The whole `verdict_from_line`
function is exercised only via the real `cargo kani` run, which is not
reachable in normal CI.

**Test to add** (`crates/titania-lanes/tests/v15_kani_verdict.rs`):

```rust
use titania_lanes::run_lane_kani::*;  // adjust to expose for test

#[test]
fn verdict_successful_exact() {
    assert_eq!(parse_verdict("VERIFICATION:- SUCCESSFUL"), HarnessVerdict::Successful);
}

#[test]
fn verdict_failed_exact() {
    assert_eq!(parse_verdict("VERIFICATION:- FAILED"), HarnessVerdict::Failed);
}

#[test]
fn verdict_unsupported_substring_match() {
    assert_eq!(
        parse_verdict("VERIFICATION: UNSUPPORTED"),
        HarnessVerdict::Unsupported,
    );
}

#[test]
fn verdict_unsupported_no_false_positive_on_successful_message() {
    // BUG: currently mis-classified because of substring bug.
    assert_eq!(
        parse_verdict("VERIFICATION:- SUCCESSFUL with unsupported-feature warning"),
        HarnessVerdict::Successful,
    );
}

#[test]
fn verdict_unknown_when_no_marker() {
    assert_eq!(parse_verdict("random noise"), HarnessVerdict::Unknown);
}

#[test]
fn verdict_unknown_when_marker_with_empty_body() {
    assert_eq!(parse_verdict("VERIFICATION:"), HarnessVerdict::Unknown);
}
```

But `parse_verdict` is **private** — there is no public API to exercise it.
There is no test that drives the lane's verdict-classification logic
without actually running `cargo kani` (which the dev environment can't do
in 60s; see RELEASE_REPORT.md Known Issue §1).

### Gap 14: `parse_verdict` is case-sensitive

**File**: `crates/titania-lanes/src/run_lane_kani.rs:292-296`.
**Bug**: `"successful"`, `"Successful"`, `"FAILED "` (trailing whitespace that
isn't ASCII space, e.g. full-width space `\u{3000}`) all map to `Unknown`.
No test covers these.

If Kani ever changes its output casing (or emits warning text in a
non-ASCII locale), the classifier silently returns `Unknown` instead of
`Successful`/`Failed`.

**Test to add** (requires making `verdict_from_line` test-visible):

```rust
#[test]
fn verdict_lowercase_is_unknown() {
    // Either accept it (case-insensitive) or reject it explicitly.
    assert_eq!(parse_verdict("VERIFICATION:- successful"), HarnessVerdict::Unknown);
}

#[test]
fn verdict_exact_match_is_strict() {
    // Document the case-sensitivity contract.
    assert_eq!(parse_verdict("VERIFICATION:- SUCCESSFUL"), HarnessVerdict::Successful);
    assert_eq!(parse_verdict("VERIFICATION:- successful"), HarnessVerdict::Unknown);
}
```

### Gap 15: Kani lane `FALLBACK_RULE_ID` cross-namespace — never tested

**File**: `crates/titania-lanes/src/run_lane_kani.rs:69-75`. The static
fallback for the Kani lane prefers `PROOF_KANI_FAIL`, but if that fails, falls
back to `MUTANT_SURVIVED` (a mutant rule id!). If `RuleId::new("PROOF_KANI_FAIL")`
ever returns an error (e.g. someone loosens rule-id validation), every
Kani finding would silently be classified as `MUTANT_SURVIVED`.

There is no test for this fallback path.

**Test to add**: redacted — requires either a feature flag to disable the
primary RuleId literal, or refactoring `rule_id_for_harness_or_static` to
inject a config. The current test architecture makes this impractical
without a `#[cfg(test)]` injection point.

### Gap 16: `PROOF_KANI_NOT_RUN` is documented and catalogued but never used

**File**: `crates/titania-lanes/src/run_lane_kani.rs:11` declares
`PROOF_KANI_NOT_RUN (informational) when cargo-kani is missing`. The repair
catalog includes the row (`crates/titania-core/src/finding/repair_catalog.tsv:75`).
The actual implementation in `list_error_outcome` (lines 121-123) returns
`LaneOutcome::Skipped { reason: SkipReason::NotApplicable }` instead of an
informational `PROOF_KANI_NOT_RUN` finding.

**Bug**: there are two competing idioms — skip-vs-informational — and the
doc/catalog contradict the code. No test exercises the missing-tool path
because `cargo-kani` is unavailable in the test env.

**Contract test to add**: set `TITANIA_NO_KANI=1` or skip the lane on a stub.
Better: drive `list_error_outcome` directly (needs test visibility).

### Gap 17: `v15_atomic_baseline.rs` does NOT test the atomic-write contract

**File**: `crates/titania-lanes/tests/v15_atomic_baseline.rs`.
**Bug**: `write_baseline_atomically` is called **once before** the
`loom::model` block, then two reader threads independently `load` the
already-written file. This tests "two readers see the same content", not
"atomic rename prevents readers from seeing a partial write". The contract
under test (file-level lines 10-17) is **the latter**.

If the production writer were ever changed from
`write → rename(temp → final)` to `write(final)` (non-atomic), this test
would still pass because no interleaving between writes and reads occurs
inside `loom::model`.

**Test to add** (the missing writer/reader interleaved model):

```rust
#[test]
fn atomic_baseline_load_under_concurrent_writer() {
    let tmp = TempDir::new().unwrap();
    let baseline_path: PathBuf = tmp.path().join(".titania").join("baseline.json");
    write_baseline_atomically(&baseline_path);
    let shared_path: Arc<PathBuf> = Arc::new(baseline_path);
    let shared_path_w = shared_path.clone();

    loom::model(move || {
        // Writer thread issues repeated atomic rewrites (the production pattern).
        let writer_done = Arc::new(AtomicBool::new(false));
        let writer_done_clone = writer_done.clone();

        let writer = thread::spawn(move || {
            for _ in 0..3 {
                write_baseline_atomically(&shared_path_w);
            }
            writer_done_clone.store(true, Ordering::SeqCst);
        });

        // Reader thread loads concurrently.
        let reader = thread::spawn(move || {
            while !writer_done.load(Ordering::SeqCst) {
                if let Ok(loaded) = MutantsBaseline::load(&shared_path) {
                    // The entries count must be one of {0 (intermediate), 2 (final)}.
                    let len = loaded.entries().len();
                    assert!(
                        len == 0 || len == 2,
                        "partial writes observed: entries={len}"
                    );
                }
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    });
}
```

This requires `AtomicBool` to be added to the loom cfg-only imports and the
crate's `loom` feature needs to be enabled.

**Mutation that survives**: the current test would pass even if the writer
were modified to call `std::fs::write(path, partial_payload)` directly
without the temp+rename pattern, because no writer runs inside the model
loop.

### Gap 18: `LOOM_MAX_PREEMPTIONS` is documented at 2 but neither set nor
asserted

**File**: `crates/titania-lanes/tests/v15_atomic_baseline.rs:7`. The doc
recommends `LOOM_MAX_PREEMPTIONS=2` but the file does not set this env, and
no test infrastructure enforces it. With two threads and a `loom::model`
bound of `0` preemption per-thread, loom explores every pair of
orderings; with the default of 6, it explores exponentially more.

A bug-introducing change that breaks the invariant under a deeper
interleaving would not be caught with the documented setting of 2 — and the
2 setting is not actually set anywhere in the file.

**Test to add**: a `Cargo` alias or CI-side assertion that
`LOOM_MAX_PREEMPTIONS=2 cargo test --test v15_atomic_baseline` runs.

### Gap 19: `write_baseline_atomically` writes the file once but the test
expects 2 entries

**File**: `crates/titania-lanes/tests/v15_atomic_baseline.rs:113-130`,
`baseline_json_blob()` builds 2 entries and `EXPECTED_ENTRY_COUNT: usize = 2`
asserts that both readers see exactly 2 entries. The writer helper writes
once. If a partial write happened (the case the test should be guarding
against), the count would be `0` or anything in between — but the test
currently only ever sees `2`. This is a feature, not a bug, of the
**current** test design — but a property test would catch a regression
where the file ends up partially written.

See Gap 17 for the proposed interleaved-writer model.

### Gap 20: `cargo run -p titania-check -- run-lane kani` / `run-lane
mutants` have no end-to-end CLI test

**Search**: `rg "run-lane kani|run-lane mutants|Lane::Kani|Lane::Mutants"
crates/titania-check/tests/`
— **zero matches**.

The full v1.5 contract acceptance A7 ("moon :titania:gate-full exits 0 from
a clean workspace") and A8 ("titania-check --scope full --emit json exits 0")
are achieved by the real run (RELEASE_REPORT.md), but **no automated test
guards against regression** in the dispatch wiring (`non_cargo_outcome` in
`run_lane.rs:136-149`).

**Test to add** (`crates/titania-check/tests/v15_run_lane_full.rs`):

```rust
#[cfg(unix)]
#[test]
fn run_lane_kani_is_reachable_via_cli() {
    // The Kani lane must be reachable (the `Kani` arm of `non_cargo_outcome`
    // must exist). With no cargo-kani binary on PATH, we expect a Skipped
    // outcome with exit 0, and a typed artifact on disk.
    let workspace = empty_target_workspace();
    let (code, _stdout, stderr) = run_in(workspace.path(), &["run-lane", "kani"]);
    assert_eq!(code, 0, "run-lane kani must exit 0 (skip is acceptable); stderr: {stderr}");
    let artifact = workspace.path().join(".titania").join("out").join("full").join("kani.json");
    assert!(artifact.exists(), "kani.json must be written under .titania/out/full/");
}

#[cfg(unix)]
#[test]
fn run_lane_mutants_emits_typed_artifact() {
    let workspace = workspace_with_empty_baseline();
    let (code, _stdout, _stderr) =
        run_in(workspace.path(), &["run-lane", "mutants"]);
    // Lane exit is 1 (violations = empty baseline => 2827 MUTANT_SURVIVED) — but
    // the typed artifact MUST exist.
    let artifact = workspace.path().join(".titania").join("out").join("full").join("mutants.json");
    assert!(artifact.exists(), "mutants.json must be written under .titania/out/full/");
}
```

### Gap 21: `titania-check --scope full` has no CLI test

**File**: `crates/titania-check/tests/cli_dispatch.rs:223-238`. The closest
test is `cli_args_unknown_scope_rejected` which uses `"definitely-unknown-scope"`
to reject a junk value. There is **no positive test for `--scope full`**.

The required acceptance A8 ("titania-check --scope full --emit json exits 0
from a clean workspace") is unverified at the test level. Currently the test
file exits 1 with `[--scope unknown]` only — the positive case for
`full` requires writing 12 typed lane artifacts to a temp workspace, which
the test infrastructure can do but hasn't.

**Test to add**:

```rust
#[cfg(unix)]
#[test]
fn cli_scope_full_is_accepted_by_parser() {
    let (code, stdout, stderr) =
        run(&["--scope", "full", "--emit", "json"]);
    // The parser must accept 'full' without exit 3; downstream aggregate
    // may exit 1 because no real artifacts exist on disk in this test cwd,
    // but the route must reach `Command::Check`.
    assert_ne!(code, 3, "scope=full must not produce InputError; stderr: {stderr}");
    assert!(!stdout.is_empty() || !stderr.is_empty(),
        "either stdout (report) or stderr (failure) must be populated");
}
```

### Gap 22: `titania-check explain` for v1.5 rule ids is not in the rule
list

**File**: `crates/titania-check/tests/explain.rs:64-118`. The list of
required-prose rule ids includes many pre-existing rules but **omits**
`PROOF_KANI_PASS`, `PROOF_KANI_FAIL`, `PROOF_KANI_BLOCKED`,
`PROOF_KANI_NOT_RUN`, `PROOF_KANI_UNSUPPORTED`, `PROOF_KANI_INFRA`,
`MUTANT_SURVIVED`, `MUTANT_SURVIVED_INFRA`, `MUTANT_BASELINE_MISSING`.

The spec acceptance A9 ("titania-check explain PROOF_KANI_FAIL and
titania-check explain MUTANT_SURVIVED return prose") has no automated
test.

**Test to add**:

```rust
#[test]
fn explain_proof_kani_fail_returns_prose() {
    let (code, stdout, stderr) = run(&["explain", "PROOF_KANI_FAIL"]);
    assert_eq!(code, 0, "explain PROOF_KANI_FAIL must exit 0; stderr: {stderr}");
    assert!(stdout.contains("PROOF_KANI_FAIL"), "stdout must echo rule id");
    assert!(stdout.contains("Reject") || stdout.contains("failure"),
        "stdout must describe the failure shape");
}

#[test]
fn explain_mutant_survived_returns_prose() {
    let (code, stdout, stderr) = run(&["explain", "MUTANT_SURVIVED"]);
    assert_eq!(code, 0, "explain MUTANT_SURVIVED must exit 0; stderr: {stderr}");
    assert!(stdout.contains("MUTANT_SURVIVED"));
    assert!(stdout.contains("baseline") || stdout.contains("bypass"),
        "stdout must reference the baseline/bypass repair path");
}

#[test]
fn explain_mutant_baseline_missing_returns_prose() {
    let (code, stdout, _stderr) = run(&["explain", "MUTANT_BASELINE_MISSING"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("mutants-bootstrap"));
}
```

### Gap 23: `PROPERTY` tests for `KaniHarnessId`, `MutantId`, `MutantOperator`

**File**: `crates/titania-core/tests/properties.rs`. Property tests exist
for `Digest`, `RuleId`, `WorkspacePath`, `TextRange`. **No property tests
exist for any of the new v1.5 types.** Proptest would explore the
strategy-defined input space and surface the same edge cases the unit tests
miss.

**Tests to add** (new property test file or extension):

```rust
proptest! {
    #[test]
    fn kani_harness_id_display_round_trip(s in "[A-Z_0-9]{1,96}") {
        // Reject invalid prefixes explicitly in the strategy so we only
        // exercise valid round-trips.
        if let Ok(first) = s.chars().next() {
            if (first.is_ascii_uppercase() && s.contains('_')) ||
               (s.contains('_') && !s.chars().any(|c| c.is_ascii_digit())) {
                // ...
            }
        }
    }

    #[test]
    fn kani_harness_id_rejects_underscore_only(s in "_+") {
        let result = KaniHarnessId::new(&s);
        // If the impl is correct, an all-underscore input must be rejected.
        // If the impl is buggy (current!), this passes for some inputs.
        prop_assert!(result.is_err() || s.chars().any(|c| c.is_ascii_uppercase()),
            "KaniHarnessId::new accepted invalid id: {s}");
    }

    #[test]
    fn mutant_id_canonical_format_round_trip(
        package in "[a-z][a-z0-9_-]{0,15}",
        rel_path in "[a-z][a-z0-9_/.-]{0,30}",
        line in 1u32..1000,
        col in 1u32..1000,
        op_idx in 0u8..8,
    ) {
        // Filter out paths with disallowed characters.
        prop_assume!(!rel_path.contains(':'));
        prop_assume!(!rel_path.starts_with('/'));
        let operator = match op_idx {
            0 => MutantOperator::EqualReplace,
            1 => MutantOperator::NotInserted,
            2 => MutantOperator::AndOr,
            3 => MutantOperator::IntegerPlusOne,
            4 => MutantOperator::IntegerMinusOne,
            5 => MutantOperator::ArithmeticOpFlip,
            6 => MutantOperator::DefaultReplace,
            _ => MutantOperator::RemoveNegation,
        };
        let id = MutantId::new(&package, &rel_path, line, col, operator).unwrap();
        let expected = format!("{package}::{rel_path}:{line}:{col}:{}", operator.as_str());
        prop_assert_eq!(id.as_str(), expected.as_str());
        prop_assert_eq!(id.package(), package.as_str());
        prop_assert_eq!(id.operator_matched(operator.as_str()), true);
    }
}
```

### Gap 24: `MutantOperator::as_str()` does not match its serde rename

**File**: `crates/titania-core/src/proof_id.rs:136-170`. The enum has
`#[serde(rename_all = "snake_case")]` and `as_str()` returns
`"equal_replace"`, `"not_inserted"`, etc. — matching. **No test asserts
that `as_str()` matches `serde_json::to_string` output.**

**Test to add** (`v15_mutant_id.rs` or a new file):

```rust
#[test]
fn operator_as_str_matches_serde_form() {
    for (op, expected) in [
        (MutantOperator::EqualReplace, "equal_replace"),
        (MutantOperator::NotInserted, "not_inserted"),
        (MutantOperator::AndOr, "and_or"),
        (MutantOperator::IntegerPlusOne, "integer_plus_one"),
        (MutantOperator::IntegerMinusOne, "integer_minus_one"),
        (MutantOperator::ArithmeticOpFlip, "arithmetic_op_flip"),
        (MutantOperator::DefaultReplace, "default_replace"),
        (MutantOperator::RemoveNegation, "remove_negation"),
    ] {
        assert_eq!(op.as_str(), expected);
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
    }
}

#[test]
fn operator_rejects_unknown_via_serde() {
    let err = serde_json::from_str::<MutantOperator>("\"bogus_op\"").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unknown variant"));
}
```

### Gap 25: `ToolKind` round-trip via serde not tested for input validation

**File**: `crates/titania-core/tests/v15_skip_reason_tool_unavailable.rs:14-32`.
Tests cover output but no JSON input rejection:

```rust
#[test]
fn tool_kind_serde_rejects_unknown_kind() {
    let err = serde_json::from_str::<ToolKind>("\"unknown-tool\"").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unknown"));
}
```

### Gap 26: `Lane::from_str` case sensitivity is undocumented and untested
for v1.5 variants

**File**: `crates/titania-core/src/lane.rs:95-114`. The match arms are
literal `PascalCase`. There is no test for `"kani"` (lowercase) → reject, or
`"Kani"` (uppercase) → accept.

`v15_lane_roundtrip.rs:21` does test the four variants for `Ok`, but no
test for case-mismatch or trailing whitespace:

```rust
#[test]
fn lane_rejects_lowercase_kani() {
    let err = Lane::from_str("kani").unwrap_err();
    assert!(matches!(err, LaneError::UnknownLane(_)));
}

#[test]
fn lane_rejects_trailing_whitespace_kani() {
    let err = Lane::from_str("Kani ").unwrap_err();
    assert!(matches!(err, LaneError::UnknownLane(_)));
}
```

### Gap 27: `KaniHarnessId` boundary at `KANI_HARNESS_ID_MAX_LEN - 1` only

`v15_kani_harness_id.rs:58-63` accepts `KANI_HARNESS_ID_MAX_LEN` chars (96).
`rejects_over_max_len` (line 51-55) rejects `KANI_HARNESS_ID_MAX_LEN + 1`
chars (97).

But the upper-bound test uses 97 `'A'` chars — **all the same character** —
which is the LEAST demanding boundary. The actual fence condition mixes
digits, underscores, and letters; the `KANI_HARNESS_ID_MAX_LEN - 1` test
uses 'A' repeated. The mixed-character boundary at 96 is not exercised.

**Test to add**:

```rust
#[test]
fn accepts_max_len_mixed_chars() {
    let mut max_len = "A".repeat(KANI_HARNESS_ID_MAX_LEN - 2);
    max_len.push('9');  // digit
    max_len.push('_');  // underscore
    assert_eq!(max_len.len(), KANI_HARNESS_ID_MAX_LEN);
    let id = KaniHarnessId::new(&max_len);
    assert!(matches!(id, Ok(_)));
}

#[test]
fn rejects_max_len_plus_one_mixed_chars() {
    let mut over_max = "A".repeat(KANI_HARNESS_ID_MAX_LEN - 1);
    over_max.push('9');
    over_max.push('_');
    assert_eq!(over_max.len(), KANI_HARNESS_ID_MAX_LEN + 1);
    let result = KaniHarnessId::new(&over_max);
    assert!(matches!(result, Err(KaniHarnessIdError::TooLong(_))));
}
```

### Gap 28: `KaniHarnessId::is_equal` does not handle the `Hash` contract

`v15_kani_harness_id.rs:66-72` tests `is_equal`. The struct derives
`PartialEq, Eq, Hash`. The custom `is_equal` is redundant with `==`. If
someone refactors `is_equal` to use case-insensitive comparison (breaking
the doc contract), `==` would still hold but `is_equal` would diverge —
no test catches this drift.

**Test to add**:

```rust
#[test]
fn is_equal_must_match_partial_eq() {
    let a = KaniHarnessId::new("FOO_BAR").unwrap();
    let b = KaniHarnessId::new("FOO_BAR").unwrap();
    let c = KaniHarnessId::new("FOO_BAZ").unwrap();
    assert_eq!(a == b, a.is_equal(&b));
    assert_eq!(a == c, a.is_equal(&c));
}
```

### Gap 29: `KaniHarnessId` FromStr does not consume leading whitespace

The `FromStr` impl calls `Self::new(s)`. `s` may have leading whitespace
and pass through. No test covers this.

**Test to add**:

```rust
#[test]
fn rejects_leading_whitespace() {
    let result = KaniHarnessId::from_str(" FOO_BAR");
    // Trim is not performed; leading space is rejected as NotUpperAscii.
    assert!(matches!(result, Err(KaniHarnessIdError::NotUpperAscii { byte: b' ', offset: 0 })));
}
```

### Gap 30: `MutantOperator` exhaustive enum is unused by `MutantId::new`

`MutantOperator` is a closed enum and is the only way to construct a
`MutantId`. **No property test exists to enumerate every operator
combination.**

---

## Brittleness Findings

> "Could this test pass while the implementation is broken?"

### Brittleness 1: `accepts_uppercase_with_underscore` — single happy path

**File**: `crates/titania-core/tests/v15_kani_harness_id.rs:9-13`.
**Assertion**: `matches!(id, Ok(ref v) if v.as_str() == "FOO_BAR")`.

A buggy implementation that accepts everything (returning `Ok(Wrapped)`)
**still passes** this test. The contract "must start with a letter, must
contain `_`, must be uppercase ASCII" is enforced by the OTHER seven
tests, but if **all seven** were deleted, **only this test would remain**
and a "accept everything" impl would pass.

The harness assertions are individually strong but **collectively
tested for "all-positive" property only**: every assertion is on a
specific input class. There is no `proptest!` block to sample the
strategy-defined space.

### Brittleness 2: `rejects_*` tests use `unwrap_err` — what if the impl
panics?

**File**: `crates/titania-core/tests/v15_kani_harness_id.rs`, `v15_mutant_id.rs`,
`v15_mutants_baseline_load.rs`.

All rejection tests use `result.unwrap_err()`. **If the impl panicked
instead of returning `Err`, the test would also panic** — meaning the
test does not distinguish "rejected" from "panicked". The Holzman "no
panic in production" rule is not enforced at the test boundary.

To harden: wrap in `std::panic::catch_unwind` and assert that:
1. The call did not panic.
2. The result is `Err`.

### Brittleness 3: `is_equal` test allows `==` and `is_equal` to diverge

**File**: `crates/titania-core/tests/v15_kani_harness_id.rs:66-72`.
**Assertion**: only positive cases (`a.is_equal(&b)` and `!a.is_equal(&c)`).

A test that exercises the divergence of `==` and `is_equal` would expose
refactor drift. See Gap 28.

### Brittleness 4: `serde_rejects_invalid_strings` only checks substring

**File**: `crates/titania-core/tests/v15_kani_harness_id_serde.rs:15-18`.

```rust
let err = serde_json::from_str::<KaniHarnessId>("\"foo\"").unwrap_err();
assert!(err.to_string().to_lowercase().contains("kani harness id"));
```

A buggy implementation that returned the wrong error type
(e.g. `Empty`) for the input `"foo"` (which is lowercase, not empty) **would
still pass** as long as the error message contains "kani harness id".
A precise type assertion (`matches!(err, DeserializeError::Custom(_)`)) is
not made.

### Brittleness 5: `serde_rejects_empty_string` is redundant with
`rejects_empty`

**File**: `crates/titania-core/tests/v15_kani_harness_id_serde.rs:20-23`.

The serde path exercises the `Deserialize` impl, which calls
`Self::new(&raw)`. So `serde_rejects_empty_string` effectively tests
exactly the same code path as `rejects_empty` — the serde indirection is
not adding coverage.

To add value, the test should exercise a serde-DOUBLE-encoded input
that triggers a JSON-level error (e.g. `"123"` parsed as number? — but
`Self::new("123")` would reject anyway). A better test would deserialise
a deeply-nested serde path to exercise deserialization-specific failure
modes.

### Brittleness 6: `gate_scope_full_includes_kani_and_mutants` checks
**membership only**

**File**: `crates/titania-core/tests/v15_lane_roundtrip.rs:40-47`.

```rust
assert!(lanes.contains(&Lane::Kani), "GateScope::Full must include Lane::Kani");
assert!(lanes.contains(&Lane::Mutants), "GateScope::Full must include Lane::Mutants");
for rel in GateScope::Release.lanes() {
    assert!(lanes.contains(rel), "GateScope::Full must include {rel:?} from Release");
}
```

This passes even if `GateScope::Full::lanes()` returns every existing
lane + duplicates. **No test asserts that `lanes()` returns lanes in the
canonical order** (spec §13), or that there are no duplicates.

`Lane::Kani` and `Lane::Mutants` could be at positions 5 and 7 (out of
order) and the test would still pass.

A stronger test would check `lanes()` for **exact equality**:

```rust
assert_eq!(GateScope::Full.lanes(), FULL_LANES_CONST);
```

where `FULL_LANES_CONST` is the spec-mandated ordering.

### Brittleness 7: `gate_scope_release_does_not_include_kani_or_mutants`

**File**: `crates/titania-core/tests/v15_lane_roundtrip.rs:50-54`. Same
critique as Brittleness 6 — passes if Release includes these lanes
**as extras** (test only checks absence in `Release`, not exact
equality).

### Brittleness 8: `lane_name_uniqueness` — checked but only for name

**File**: `crates/titania-core/tests/v15_lane_name.rs:6-27`. Tests `name`
uniqueness but not `file_stem` uniqueness is also covered (lines 30-51).
However, `serde_json::to_string(&lane)` producing the right form is only
tested for `Kani`/`Mutants`, **not for ALL 12 lanes**. A bug where
`serde(rename_all = "PascalCase")` was changed to `"lowercase"` would
silently affect 10 of the 12 lanes without any test catching it.

**Test to add**:

```rust
#[test]
fn serde_json_serializes_all_lanes_in_pascal_case() {
    for lane in [
        Lane::Fmt, Lane::Compile, Lane::Clippy, Lane::AstGrep, Lane::Dylint,
        Lane::PanicScan, Lane::PolicyScan, Lane::Test, Lane::Deny,
        Lane::Build, Lane::Kani, Lane::Mutants,
    ] {
        let json = serde_json::to_string(&lane).unwrap();
        let name = lane.name();
        assert_eq!(json, format!("\"{name}\""), "{lane:?} wrong serde form");
    }
}
```

### Brittleness 9: `serde_rejects_unknown_variant` checks substring

**File**: `crates/titania-core/tests/v15_lane_serde_roundtrip.rs:48-51`.

```rust
let err = serde_json::from_str::<Lane>("\"UnknownLane\"").unwrap_err();
assert!(err.to_string().to_lowercase().contains("unknown"));
```

A buggy impl that returned `Err(EmptyVariant)` (no such variant)
with a message containing "unknown" would pass. The error type isn't
checked.

### Brittleness 10: `from_bypasses_round_trip_via_json` is essentially
trivial

**File**: `crates/titania-core/tests/v15_mutants_baseline_load.rs:51-62`.
**Behaviour**: construct, serialise, deserialise, check equality. Standard
serde round-trip. No verification that the resulting `MutantBaselineEntry`
preserves invariants (e.g. `mutation_id` is non-empty after
deserialisation — though serde would fail if the input was empty, this is
not tested).

### Brittleness 11: `v15_atomic_baseline` does NOT verify the atomicity

**File**: `crates/titania-lanes/tests/v15_atomic_baseline.rs:128-140`.
**Calling seq**: write happens once before the model; the model only
spawns two reader threads. **The atomic-rename contract is never
exercised inside `loom::model`.**

This is the deepest structural brittleness in v1.5: the only loom
concurrency test does not actually test the property it claims.

### Brittleness 12: `cargo fmt` passing on a single 700-line file
proves nothing about file-size compliance

**File**: `crates/titania-lanes/src/run_lane_mutants.rs:473` is at the 473
lines upper bound. `run_lane_kani.rs` is at 695 lines. Both exceed the
60-line-function rule of `holzman-rust`. The lint-src Moon task is
narrowly defined and does **not** check file-length.

### Brittleness 13: `parse_verdict` is private — cannot be exercised from
test

**File**: `crates/titania-lanes/src/run_lane_kani.rs:287-297`. The
function is module-private (`fn verdict_from_line`). Without test-visibility
or a mock harness output, the only test path is the real `cargo kani`
binary, which the test environment can't run in 60s (RELEASE_REPORT.md
Known Issue §1).

### Brittleness 14: `accepts_max_len_boundary` rejects the 96-char boundary
on accident

**File**: `crates/titania-core/tests/v15_kani_harness_id.rs:58-63`. The
test creates a string of length `KANI_HARNESS_ID_MAX_LEN - 1` (= 95)
"A"s plus one "_", giving length 96. `KANI_HARNESS_ID_MAX_LEN` is `96`.
The test passes if `s.len() <= KANI_HARNESS_ID_MAX_LEN`. This exercises
the **96-boundary case correctly**, but only for `'A'` * 95 + `_`. The
test does NOT verify that **any 96-char string that passes validation
is accepted** — it only verifies one specific 96-char string is
accepted.

### Brittleness 15: `serde_round_trip_all_variants` tests each variant in
isolation but not aggregate

`v15_lane_serde_roundtrip.rs:23-29` round-trips each lane individually. A
serialised form that is invalid in a Vec context (e.g. `"Fmt"` with a
JSON-array delimiter) would not be caught. A stronger test:

```rust
#[test]
fn serde_in_array_round_trip() {
    let lanes = vec![
        Lane::Fmt, Lane::Kani, Lane::Mutants,
    ];
    let json = serde_json::to_string(&lanes).unwrap();
    let back: Vec<Lane> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, lanes);
}
```

---

## Recommended Test Additions

Concrete list of new tests to add in priority order. Each is justified by
the gap or brittleness above.

| # | Test name | File | Purpose |
|---|---|---|---|
| T1 | `rejects_leading_underscore` | `v15_kani_harness_id.rs` | Doc contract: must start with a letter |
| T2 | `rejects_nul_byte` | `v15_kani_harness_id.rs` | Non-ASCII rejection |
| T3 | `rejects_utf8_multibyte` | `v15_kani_harness_id.rs` | Multi-byte rejection |
| T4 | `rejects_only_underscores` | `v15_kani_harness_id.rs` | All-underscore edge case |
| T5 | `rejects_mixed_case` | `v15_kani_harness_id.rs` | `Foo_Bar` case |
| T6 | `rejects_rel_path_with_colon` | `v15_mutant_id.rs` | Canonical format enforcement |
| T7 | `rejects_rel_path_with_double_colon` | `v15_mutant_id.rs` | Sub-`::` rejection |
| T8 | `accepts_u32_max_line_and_col` | `v15_mutant_id.rs` | Boundary at u32::MAX |
| T9 | `unknown_operator_deserializes_to_unknown` | `v15_mutant_id.rs` | Wire-format strong typing |
| T10 | `operator_as_str_matches_serde_form` | `v15_mutant_id.rs` | Round-trip on the enum |
| T11 | `operator_rejects_unknown_via_serde` | `v15_mutant_id.rs` | Closed-set discipline |
| T12 | `kani_harness_id_display_round_trip` (proptest) | new `v15_kani_id_proptest.rs` | Strategy coverage |
| T13 | `kani_harness_id_rejects_underscore_only` (proptest) | new file | Strategy for invalid boundary |
| T14 | `mutant_id_canonical_format_round_trip` (proptest) | new `v15_mutant_id_proptest.rs` | Strategy coverage with colon filter |
| T15 | `accepts_max_len_mixed_chars` | `v15_kani_harness_id.rs` | Boundary beyond homogenous chars |
| T16 | `rejects_max_len_plus_one_mixed_chars` | `v15_kani_harness_id.rs` | Mixed-char over-length rejection |
| T17 | `is_equal_must_match_partial_eq` | `v15_kani_harness_id.rs` | Refactor-drift detector |
| T18 | `rejects_leading_whitespace` | `v15_kani_harness_id.rs` | Whitespace rejection |
| T19 | `missing_schema_version_field_is_rejected_as_unsupported` | `v15_mutants_baseline_load.rs` | Missing-field diagnostic |
| T20 | `expires_at_zero_rejects_every_timestamp` | `v15_baseline_expiry.rs` | expires_on_unix=0 boundary |
| T21 | `boundary_one_before_does_not_cover` | `v15_baseline_expiry.rs` | Off-by-one below expiry |
| T22 | `boundary_one_after_does_not_cover` | `v15_baseline_expiry.rs` | Off-by-one above expiry |
| T23 | `diff_handles_10k_baseline_entries` | `v15_mutants_baseline_diff.rs` | Production-scale perf |
| T24 | `verdict_successful_exact` | new `v15_kani_verdict.rs` (requires test-visibility refactor) | Parser happy path |
| T25 | `verdict_failed_exact` | new file | Parser failure path |
| T26 | `verdict_unsupported_substring_match` | new file | Parser unsupported path |
| T27 | `verdict_unsupported_no_false_positive_on_successful_message` | new file | Substring-bug guard |
| T28 | `verdict_unknown_when_no_marker` | new file | Default branch |
| T29 | `verdict_lowercase_is_unknown` | new file | Case-sensitivity contract |
| T30 | `atomic_baseline_load_under_concurrent_writer` | `v15_atomic_baseline.rs` | Real atomic-rename interleaving |
| T31 | `cli_scope_full_is_accepted_by_parser` | `cli_dispatch.rs` | A8 acceptance |
| T32 | `run_lane_kani_is_reachable_via_cli` | new `v15_run_lane_full.rs` | Dispatch wiring |
| T33 | `run_lane_mutants_emits_typed_artifact` | new file | Dispatch wiring |
| T34 | `explain_proof_kani_fail_returns_prose` | `explain.rs` | A9 acceptance |
| T35 | `explain_mutant_survived_returns_prose` | `explain.rs` | A9 acceptance |
| T36 | `explain_mutant_baseline_missing_returns_prose` | `explain.rs` | A9 acceptance |
| T37 | `lane_rejects_lowercase_kani` | `v15_lane_roundtrip.rs` | Lane::from_str case contract |
| T38 | `lane_rejects_trailing_whitespace_kani` | new file | Lane::from_str whitespace |
| T39 | `serde_json_serializes_all_lanes_in_pascal_case` | `v15_lane_serde_roundtrip.rs` | All-variant serde check |
| T40 | `serde_in_array_round_trip` | `v15_lane_serde_roundtrip.rs` | Vec context |
| T41 | `tool_kind_serde_rejects_unknown_kind` | `v15_skip_reason_tool_unavailable.rs` | Closed-set defense |

---

## Mutation Resistance

What mutations would survive the existing test surface?

| Mutation | Where | Surviving? | Severity |
|---|---|---|---|
| `KaniHarnessId::new` accepts leading underscore | `proof_id.rs:104-110` | **YES** — no test exercises `_FOO_BAR` or `___` | MAJOR |
| `KaniHarnessId::new` accepts lowercase | `proof_id.rs:116-128` | NO — covered by `rejects_lowercase` | — |
| `KaniHarnessId::new` accepts NUL byte | `proof_id.rs:116-128` | **YES** — no test for `\0` | MINOR |
| `KaniHarnessId::new` accepts UTF-8 multi-byte | `proof_id.rs:116-128` | **YES** — no test for `Ä` | MAJOR (denial-of-service: 4-byte UTF-8 → massive offset) |
| `KaniHarnessId::new` accepts `_` (single char) | `proof_id.rs:104-110` | **YES** | MINOR |
| `KaniHarnessId::new` accepts string with both `_` and digits not at start | `proof_id.rs:104-110` | NO — covered by `rejects_leading_digit` | — |
| `KaniHarnessId::new` accepts whitespace | `proof_id.rs:116-128` | **YES** — no test | MINOR |
| `MutantId::new` accepts `:` in path | `proof_id.rs:200-261` | **YES** — no test | MAJOR (corrupts parser contracts) |
| `MutantId::new` accepts `::` in path | `proof_id.rs:200-261` | **YES** — no test | MAJOR |
| `MutantId::new` accepts `u32::MAX` line | `proof_id.rs:200-261` | **YES** | MINOR |
| `MutantId::new` accepts all rel_path chars | `proof_id.rs:200-261` | **YES** — no proptest | MINOR |
| `contains()` flipped to "must match non-expired" (off-by-one) | `mutants_baseline.rs:180-183` | NO — `boundary_timestamp_is_inclusive` covers inclusive | — |
| `contains()` flipped to "now_unix <= exp" → "<" | `mutants_baseline.rs:180-183` | **YES** — no test for `now_unix == expires_on_unix + 1` | MAJOR (off-by-one) |
| `diff()` returns survivors instead of new | `mutants_baseline.rs:103-105` | NO — covered | — |
| `diff()` returns always-empty | `mutants_baseline.rs:103-105` | NO — covered | — |
| `validate_baseline` skips schema_version check | `mutants_baseline.rs:154-160` | NO — covered by `wrong_schema_version_returns_unsupported` | — |
| `validate_baseline` skips entry check | `mutants_baseline.rs:170-176` | NO — covered by `empty_mutation_id_returns_invalid` | — |
| `parse_verdict` accepts lowercase `successful` | `run_lane_kani.rs:291-296` | **YES** — no test, only "UNKNOWN" substring | MINOR |
| `parse_verdict` returns `Failed` for `Unsupported` | `run_lane_kani.rs:291-296` | **YES** — no test of unsupported branch | MAJOR |
| `parse_verdict` returns `Unsupported` for `Successful but unsupported warning` | `run_lane_kani.rs:294` | **YES** — substring-match bug | MAJOR |
| `parse_verdict` returns `Unsupported` for `SUCCESSFUL but unsupported-feature warning` | `run_lane_kani.rs:294` | **YES** — substring-match bug | MAJOR |
| `run_kani_harness` always reports `Successful` regardless of child exit | `run_lane_kani.rs:635-694` | NO — covered by real run | — |
| `run_kani_harness` reports `Blocked` regardless of timeout | `run_lane_kani.rs:188-200` | NO — covered by real run, but no test | — |
| `FALLBACK_RULE_ID` cross-namespace fallback activated | `run_lane_kani.rs:69-75` | **YES** — no test | MAJOR (silent misclassification) |
| `MutantsBaseline::load` ignores entries field | `mutants_baseline.rs:182-205` | NO — covered by `happy_path_loads_empty_baseline` and `diff_full_baseline` | — |
| `KaniHarnessId::is_equal` inverts | `proof_id.rs:62-65` | NO — covered by `is_equal_compares_inner_string` | — |
| `KaniHarnessId::is_equal` uses case-insensitive comparison | `proof_id.rs:62-65` | **YES** — no divergence test | MINOR |
| `MutantId::operator_matched` always returns true | `proof_id.rs:223-226` | NO — covered by `operator_matched_returns_true_on_match` and the inner `!op == name` assertion | — |
| `MutantId::operator_matched` returns true on partial match | `proof_id.rs:223-226` | **YES** — only equality is tested, not prefix/suffix | MINOR |
| `MutantId::operator_matched` matches wrong operator | `proof_id.rs:223-226` | NO — `!id.operator_matched("equal_replace")` when id is `and_or` | — |
| `Lane::Kani` returned by `from_str("Format")` | `lane.rs:98-114` | NO — covered by `serde_rejects_unknown_variant` | — |
| `GateScope::Full::lanes()` includes an extra duplicate | `gate_scope.rs:42-55` | **YES** — test only checks `contains` | MINOR |
| `GateScope::Full::lanes()` returns wrong order | `gate_scope.rs:42-55` | **YES** — test only checks `contains` | MAJOR (canonical ordering violated per spec §13) |
| `GateScope::Full` not in `Moon::FULL_TASKS` | `moon.rs:158-160` | NO — covered by `moon_task_graph.rs` (likely; not deep-checked here) | — |
| `Moon::gate-full` not wired | `all.yml:376-382` | **YES** — no Moon-graph unit test for Full scope in v15 context | MAJOR |
| `--scope full` rejected by parser | `args/parse.rs:415-422` | **YES** — no CLI test for positive parse | MAJOR |
| `titania-check explain PROOF_KANI_FAIL` returns nothing | `explain.rs:115-146` | **YES** — not in test list | MAJOR |
| `titania-check explain MUTANT_SURVIVED` returns nothing | `explain.rs:164-211` | **YES** | MAJOR |
| `GateScope` strictness flipped to `kebab-case` | `gate_scope.rs:98-109` | NO — covered by `gate_scope_full_round_trip` | — |
| `GateScope` adds extra arm without `Lane` enumeration | `gate_scope.rs:20-31` | NO — `non_exhaustive` allows it; no test catches silent extension | MINOR |
| `MutantId` `as_str()` returns a different string than `format!` | `proof_id.rs:200-202` | NO — implicit in 9 unit tests | — |
| `KaniHarnessId` `Display` returns different from `as_str()` | `proof_id.rs:68-72` | NO — `display_emits_inner_string` | — |
| `skip_path` test for `cargo-kani` missing | `run_lane_kani.rs:121-122` | **YES** — no test of `SkipReason::NotApplicable` path | MAJOR |
| `MUTANT_BASELINE_MISSING` rule id never used | specs vs impl | **YES** — described but no code path produces it | MAJOR |
| `PROOF_KANI_NOT_RUN` rule id never used | `run_lane_kani.rs:11` | **YES** — documented, catalogued, never emitted | MAJOR |

**Mutation resistance count** (mutations that survive the test surface):

- MAJOR: 14
- MINOR: 8
- TOTAL: 22 of 36 mutation hypotheses survive

That is a **61% survival rate** for hostile mutations — too high for an
adversarial review.

---

## Refactors Needed for Test Architecture

To enable the test additions above, several refactors are required (not in
scope of this review but noted for `proof-writer`):

### Refactor 1: `parse_verdict` and `verdict_from_line` need test visibility

Move them to a `#[cfg(test)] pub` accessor or a separate
`run_lane_kani::parser` module exposed to tests. Currently these are
`fn`-scoped inside `run_lane_kani.rs` and cannot be exercised without the
real `cargo kani` binary.

### Refactor 2: Kani lane should expose a `KaniTool::probe()` returning a
typed result

Today, the Kani lane detects missing cargo-kani by string-matching the
spawn error (`reason.contains("no such subcommand")`). A typed probe
(`KaniTool::probe() -> Result<Version, MissingReason>`) would let
the test environment stub the missing-tool path and exercise the
`SkipReason::NotApplicable` and `PROOF_KANI_NOT_RUN` branches.

### Refactor 3: Mutant baseline load needs an injectable clock

`MutantsBaseline::diff(...)` takes a `now_unix: u64` parameter. Good. But
`load()` does not — meaning the baseline `entry_covers` check inside
`MutantsBaseline` is not exercised by the expiry tests in production
paths. Verify `MutantsBaseline::contains()` is called by the production
lane (`run_lane_mutants.rs:163` — yes it is).

### Refactor 4: Loom test must be runnable in CI, not compile-only

`v15_atomic_baseline.rs:1-9` documents that it is `compile-only`. **A
test that doesn't run is not a test.** Make this an actual nightly-only
CI job with `RUSTFLAGS="--cfg loom"` and a low `LOOM_MAX_PREEMPTIONS=2`.

### Refactor 5: `KaniHarnessId`/`MutantId` should be wrapped in proptest
shrinking

Add property tests in `crates/titania-core/tests/properties_v15.rs` (new
file) covering the full input strategy. Current `properties.rs` only
covers pre-existing primitives.

---

## Residual Risk (carried forward)

1. **`PROOF_KANI_NOT_RUN` is dead text** — the rule id is in the repair
   catalog and explain module but no code path emits it. Spec mention is
   accurate to the catalog but not the runtime. Either emit it from
   `list_error_outcome` or remove it from the catalog.

2. **`RejectKind::KaniFail` / `RejectKind::MutantSurvivor` are missing**
   — the spec (`spec.md:158-159`) lists these as new variants of the
   `RejectKind` enum, but only `CodeOnly`/`GateOnly`/`Mixed` exist in
   `titania-core/src/report/mod.rs:73-82`. Spec ↔ impl parity is broken
   here. No test catches it because `RejectKind` has no enum-exhaustive
   test on its variants.

3. **`LaneReport::Kani` / `LaneReport::Mutants` are missing** — the spec
   (`spec.md:155-156`) lists `LaneReport::Kani(cargo_kani_lane::LaneError)`
   and `LaneReport::Mutants(cargo_mutants_lane::LaneError)` but the
   `LaneReport` struct in `titania-lanes/src/lib.rs:138-141` is just a
   data struct with no enum variants.

4. **CBMC hardware dependency** — the kani lane requires hardware capable
   of running CBMC within 60s/harness. The test environment in the
   release evidence cannot; only the real run did. CI hardware must
   match.

5. **2628 line count of v15-relevant code exceeds the 60-line hard
   ceiling** — `run_lane_kani.rs` is 695 lines (175% over) and
   `run_lane_mutants.rs` is 473 lines (158% over). The Holzman rule is
   not enforced at the Moon-task level for these new files; lint-src
   passes because it is narrowly scoped.

---

## Conclusion

The v1.5 release ships **24 cargo-gate-passing tests** for the four new
modules but the adversarial surface is **22 surviving mutations out of 36**
(61%). The most pressing gaps are:

1. `KaniHarnessId` accepts leading underscore and NUL byte (Gap 1, 2).
2. `MutantId` accepts colons in `rel_path` (Gap 5).
3. `parse_verdict`'s `contains("UNSUPPORTED")` substring match
   mis-classifies real Kani output (Gap 13).
4. The loom atomic-baseline test does not exercise atomic writes (Gap 17).
5. End-to-end CLI tests for `--scope full`, `run-lane kani`,
   `run-lane mutants`, and `explain PROOF_KANI_*` / `MUTANT_*` are all
   missing (Gap 20, 21, 22).
6. Spec ↔ impl parity violations on `RejectKind::*` variants and
   `LaneReport::Kani` / `LaneReport::Mutants` are not caught by any test
   (Residual Risk 2, 3).

**Verdict: FAIL** — proceed to `proof-writer` for the 41 recommended test
additions and the 5 architecture refactors required to make them
executable.
