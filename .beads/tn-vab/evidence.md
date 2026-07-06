# tn-vab — Evidence

## Problem

Dylint lane artifacts serialized by `artifact_writer.rs` wrapped `LaneFailure` in a `failure` key:
```json
{"variant":"failed","failure":{"infra_failure":{"tool":"...","reason":"..."}}}
```

But `LaneOutcome` uses `#[serde(tag = "variant")]` and expects:
```json
{"variant":"failed","infra_failure":{"tool":"...","reason":"..."}}
```

This caused `InputError: failed to parse outcome: unknown variant 'failed'` when the artifact reader deserialized `LaneOutcome` from the artifact.

## Root Cause

`ArtifactOutcome` was an intermediate struct with field `failure: Option<&'a LaneFailure>` that serialized with a `failure` wrapper key. The artifact reader deserializes `outcome` as `LaneOutcome` directly, which does not expect the wrapper.

## Fix

Removed `ArtifactOutcome` indirection entirely. `LaneArtifact` now holds `outcome: &'a LaneOutcome` directly, so serialization uses `LaneOutcome`'s built-in serde attributes (tagged enum with `variant` key).

## Files Changed

| File | Change |
|------|--------|
| `crates/titania-lanes/src/artifact_writer.rs` | Replaced `ArtifactOutcome<'a>` with `&'a LaneOutcome` in `LaneArtifact`. Removed `ArtifactOutcome` struct, impl block, and `From<&LaneOutcome>` impl (68 lines deleted). |
| `crates/titania-aggregate/tests/artifact_reader.rs` | Added `failed_artifact_json()` helper, `failed_outcome_roundtrips_correctly()` test (writes correct shape → reads back as `LaneOutcome::Failed`), and `broken_failure_wrapper_key_does_not_deserialize()` test (documents old broken shape must fail). |
| `crates/titania-lanes/tests/clippy_command.rs` | Updated `clippy_command_without_cargo_records_infra_failure` to assert `outcome.infra_failure` instead of `outcome.failure.infra_failure`. |
| `crates/titania-lanes/tests/fmt_lane.rs` | Updated `fmt_without_cargo_records_infra_failure` to assert `outcome.infra_failure` instead of `outcome.failure.infra_failure`. |

## Verification

### Artifact writer tests
```
cargo test -p titania-lanes --test artifact_writer
→ 10 passed
```

### Artifact reader tests (including new roundtrip)
```
cargo test -p titania-aggregate --test artifact_reader
→ 12 passed (10 existing + 2 new: failed_outcome_roundtrips_correctly, broken_failure_wrapper_key_does_not_deserialize)
```

### Lane tests
```
cargo test -p titania-lanes
→ 291 passed
```

### Full affected crate suite
```
cargo test -p titania-lanes -p titania-aggregate -p titania-core
→ 491 passed
```

### killer_demo status
```
cargo test -p titania-check --test killer_demo
→ 3 passed (B12-B14 integration tests), 12 failed
```

The 12 remaining failures are **not** dylint artifact parse errors. They map to:
- **Root Cause #1** (tn-dzp): `check` command doesn't run lanes → gate failures instead of findings
- **Root Cause #2** (tn-z3y): Clippy normalizer not wired → wrong rule_id
- **Root Cause #4**: cargo-dylint binary unavailable → gate failure (expected, not a code bug)

No InputError about `unknown variant 'failed'` appears in any test output.

## Acceptance

- ✅ Focused artifact serialization/deserialization test exits 0
- ✅ `killer_demo` no longer reports dylint artifact parse/InputError as a failure reason
- ✅ Remaining failures map to tn-dzp/tn-z3y/cargo-dylint availability
- ✅ No unrelated files edited (only artifact_writer.rs + 3 test files)
