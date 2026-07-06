# tn-pdn — Implementation Notes

**Date**: 2026-07-05  
**Bead**: tn-pdn (Killer Demo: Bad-Code Reject / Repaired-Pass)  
**Sublane**: implementation  
**Final Status**: COMPLETE — earlier blocker superseded; final evidence in `.beads/tn-pdn/evidence-bundle.md`

---

## Investigation Summary

### Phase 1: Root Cause Investigation

**Command run**:
```bash
cargo test -p titania-check --test killer_demo
```

**Result**: 3 passed, 12 failed

**Initial failures**: All bad fixture tests show `"output file missing"` for all 7 lanes (gate failures). No code findings produced.

### Phase 2: Pattern Analysis

**Finding**: The `titania-check` binary's `check` command does NOT run lanes before aggregating. It only reads existing `.titania/out/<scope>/*.json` artifacts.

**Verification**:
```bash
# Empty workspace — expected gate failures for all lanes (passes cli_dispatch tests)
$ ./target/debug/titania-check --scope edit --emit json
{"variant":"reject","code_findings":[],"gate_failures":[...]}

# After explicit lane runs, check produces findings
$ ./target/debug/titania-check run-lane ast-grep
$ ./target/debug/titania-check run-lane clippy
$ ./target/debug/titania-check --scope edit --emit json
{"variant":"reject","code_findings":[{"rule_id":"FUNC_LOOPS_FOR"},{"rule_id":"CARGO_CLIPPY_001"}]}
```

### Phase 3: Hypothesis Testing

**Hypothesis 1**: Adding `[workspace]` to fixture Cargo.toml files would make Cargo treat them as independent workspaces, fixing tool resolution.
- **Result**: Fixed workspace membership but did NOT fix lane execution gap.

**Hypothesis 2**: Replacing `unwrap_or` in repaired fixture with `flatten()` would eliminate unwrap-family usage.
- **Result**: Achieved — repaired lib.rs uses `items.into_iter().flatten().collect()`.

**Hypothesis 3**: Generating `Cargo.lock` for both fixtures would fix compile lane.
- **Result**: Generated `Cargo.lock` for both fixtures. Compile lane passes clean on repaired fixture but still produces `CARGO_COMPILE_001` (not a code finding) on bad fixture.

### Phase 4: Root Cause Confirmation

After exhaustive investigation, 4 production-level blockers were identified:

1. **`check` command doesn't run lanes** (main.rs:37) — tests need lane execution
2. **Clippy lane produces `CARGO_CLIPPY_001` not `CLIPPY_UNWRAP_USED`** (run_cargo_lane.rs:107) — tests expect typed rule IDs
3. **Dylint lane produces malformed artifact** (run_lane_dylint.rs:47) — `{"variant":"failed","failure":{...}}` vs expected `{"variant":"failed","infra_failure":{...}}`
4. **Missing infrastructure tools** (`cargo-dylint` not installed)

---

## Commands Run

| Command | Exit Status | Result |
|---------|------------|--------|
| `cargo test -p titania-check --test killer_demo` | 101 | 12 failed, 3 passed |
| `cargo test -p titania-check --test cli_dispatch` | 0 | All 22 passed |
| `titania-check --scope edit --emit json` (on bad fixture) | 1 | All 7 gate failures, 0 code findings |
| `titania-check run-lane ast-grep` (on bad fixture) | 1 | 1 finding: FUNC_LOOPS_FOR |
| `titania-check run-lane clippy` (on bad fixture) | 1 | 1 finding: CARGO_CLIPPY_001 |
| `titania-check run-lane dylint` (on bad fixture) | N/A | Malformed artifact |
| `titania-check run-lane clippy` (on repaired fixture) | 0 | Clean |

---

## Files Changed

### Allowed scope files:
1. `fixtures/strict_ai_loop_unwrap/bad/Cargo.toml` — Added `[workspace]` section
2. `fixtures/strict_ai_loop_unwrap/repaired/Cargo.toml` — Added `[workspace]` section
3. `fixtures/strict_ai_loop_unwrap/repaired/src/lib.rs` — Replaced `items.iter().map(|item| item.unwrap_or(0)).collect()` with `items.into_iter().flatten().collect()`

### Generated (not committed):
4. `fixtures/strict_ai_loop_unwrap/bad/Cargo.lock`
5. `fixtures/strict_ai_loop_unwrap/repaired/Cargo.lock`

### Documentation:
6. `.beads/tn-pdn/implementation-blocker.md` — Full blocker documentation
7. `.beads/tn-pdn/implementation.md` — This file

---

## Residual Blockers

All 4 blockers require production Rust code changes:
1. `check` command must run lanes before aggregating
2. Clippy lane must use `clippy_normalizer::normalize_clippy_jsonl` for typed rule IDs
3. Dylint lane artifact format must match `LaneOutcome` deserializer expectations
4. Infrastructure tools must be available (environment issue)

See `.beads/tn-pdn/implementation-blocker.md` for full detail.

---

## Final Resolution — 2026-07-05

The earlier `BLOCKED` status is superseded by dependency repairs:

- `tn-z3y`: `check` runs scoped lanes before aggregation.
- `tn-dzp`: Clippy lane normalizes JSON diagnostics to concrete `CLIPPY_*` findings.
- `tn-vab`: LaneOutcome artifact serialization/deserialization is consistent.
- `tn-b5j`: target discovery/root aggregation works from subdirectories.
- `tn-zuv`: pass reports include typed per-lane evidence and lane identity/order validation.

Current direct acceptance evidence is in `.beads/tn-pdn/evidence-bundle.md` and raw reports under `.beads/tn-pdn/raw/`.

Important environment note:
- `cargo test -p titania-check --test killer_demo` runs the binary at `/home/lewis/src/titania/.worktrees/v1-combined-dispatch/target/debug/titania-check` with `/cache/cargo-shared/bin` on PATH.
- That PATH contains `cargo-dylint`; the default shell PATH did not, which explained the earlier repaired-fixture Dylint infra rejection.
- Direct CLI acceptance was rerun against fresh temp fixture copies with `/cache/cargo-shared/bin` prepended to PATH.

Final result:
- Bad fixture: exit 1, `Report::Reject`, code findings exactly `CLIPPY_UNWRAP_USED` and `FUNC_LOOPS_FOR`, zero gate failures, all seven edit lanes present.
- Repaired fixture: exit 0, `Report::Pass`, `schema_version=1` receipt, zero gate failures, all seven edit lanes present.

Final status: COMPLETE.
