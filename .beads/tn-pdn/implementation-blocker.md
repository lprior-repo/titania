# tn-pdn — Implementation Blocker

**Date**: 2026-07-05  
**Bead**: tn-pdn (Killer Demo: Bad-Code Reject / Repaired-Pass)  
**Status**: RESOLVED/SUPERSEDED — dependency repairs landed; final evidence in `.beads/tn-pdn/evidence-bundle.md`

---

## Summary

`cargo test -p titania-check --test killer_demo` fails 12/15 tests. The failures are caused by **four independent production-level gaps**, none of which can be fixed within the allowed write scope (5 files: 2 fixtures × 2 files + killer_demo.rs).

---

## Root Cause #1: `check` command does NOT run lanes

**File**: `crates/titania-check/src/main.rs` (line 37)  
**Contract says**: "The tool: 1. Parses `--scope edit` → runs the 7 Edit lanes"  
**Production does**: `check` dispatches to `aggregate_scope` which calls `aggregate::report_json` — this reads existing `.titania/out/<scope>/*.json` artifacts only. It NEVER executes lanes.

**Evidence**:
```bash
# Bad fixture: all 7 lanes fail with "output file missing"
$ ./target/debug/titania-check --scope edit --emit json
{"variant":"reject","code_findings":[],"gate_failures":[...7 entries...]}

# Only after running lanes explicitly does check produce findings:
$ ./target/debug/titania-check run-lane ast-grep   # writes .titania/out/edit/ast-grep.json
$ ./target/debug/titania-check run-lane clippy     # writes .titania/out/edit/clippy.json
$ ./target/debug/titania-check --scope edit --emit json  # NOW has findings
```

**Fix required**: `main.rs` line 37 must call `run_lane` for each lane before `aggregate_scope`. This is a production code change.

---

## Root Cause #2: Clippy lane produces `CARGO_CLIPPY_001` not `CLIPPY_UNWRAP_USED`

**File**: `crates/titania-lanes/src/run_cargo_lane.rs` (line 107)  
**Contract says**: `CLIPPY_UNWRAP_USED` finding from clippy normalizer  
**Production does**: `record_command_result` pushes `Finding::new(rule.clone(), ...)` where `rule` is `CargoLane::Clippy.rule()` = `"CARGO_CLIPPY_001"`. The clippy JSONL normalizer (`clippy_normalizer.rs`) is never invoked.

**Evidence**:
```bash
$ ./target/debug/titania-check run-lane clippy  # on bad fixture
1 finding(s)

$ cat .titania/out/edit/clippy.json
{
  "outcome": {
    "variant": "findings",
    "findings": [{
      "rule_id": "CARGO_CLIPPY_001",
      "lane": "Clippy",
      "message": "Checking strict_ai_loop_unwrap_bad v0.1.0 ..."
    }]
  }
}
```

Expected `rule_id: "CLIPPY_UNWRAP_USED"` but got `"CARGO_CLIPPY_001"`.

**Fix required**: `run_cargo_lane.rs` must invoke `clippy_normalizer::normalize_clippy_jsonl` on the command output and use the normalized findings instead of the generic `CARGO_CLIPPY_001` finding.

---

## Root Cause #3: Dylint lane produces malformed artifact

**File**: `crates/titania-lanes/src/run_lane_dylint.rs` (line 47-50)  
**Issue**: `LaneFailure::Infra { ... }` is serialized as:
```json
{"variant": "failed", "failure": {"infra_failure": {"tool": "...", "reason": "..."}}}
```

But `LaneOutcome` deserializer (at `titania-core/src/outcome.rs:121`) expects:
```json
{"variant": "failed", "infra_failure": {"tool": "...", "reason": "..."}}
```

The `failure` wrapper key is unexpected. The artifact reader at `titania-aggregate/src/artifact_reader.rs:103` tries `serde_json::from_value::<LaneOutcome>(artifact.outcome)` which fails with:
```
unknown variant `failed`, expected one of `infra_failure`, `tool_failure`, `resource_failure`, `suspicious_failure`
```

**Evidence**:
```bash
$ ./target/debug/titania-check run-lane dylint  # on bad fixture
lane failed: Infra { tool: "cargo-dylint", reason: "cargo-dylint binary is unavailable" }

$ cat .titania/out/edit/dylint.json
{"outcome":{"variant":"failed","failure":{"infra_failure":{"tool":"cargo-dylint","reason":"cargo-dylint binary is unavailable"}}}}

$ ./target/debug/titania-check --scope edit --emit json
InputError: aggregate artifact read failed: input error for lane Dylint: failed to parse outcome for Dylint: unknown variant `failed`, expected one of `infra_failure`, `tool_failure`, `resource_failure`, `suspicious_failure`
```

**Fix required**: The artifact writer must serialize `LaneOutcome` directly (without the extra `failure` wrapper), or the `run_cargo` outcome construction must use a format compatible with `LaneOutcome` deserialization.

---

## Root Cause #4: Missing infrastructure tools

`cargo-dylint` and the `titania-ast-grep` binary are not installed on this system. This causes:
- Dylint lane: always fails with "binary unavailable"
- AstGrep lane: works (uses embedded binary) but may fail in other environments

**Impact**: The test needs all 7 lanes to succeed. Dylint failure becomes a gate failure, not a clean outcome.

---

## Files Changed in This Session

1. `fixtures/strict_ai_loop_unwrap/bad/Cargo.toml` — Added `[workspace]` section
2. `fixtures/strict_ai_loop_unwrap/repaired/Cargo.toml` — Added `[workspace]` section  
3. `fixtures/strict_ai_loop_unwrap/repaired/src/lib.rs` — Replaced `items.iter().map(|item| item.unwrap_or(0)).collect()` with `items.into_iter().flatten().collect()` (no unwrap-family usage)
4. Generated `Cargo.lock` for both fixtures (not committed)

These fixture fixes were necessary per Main's interrupt but are insufficient to reach GREEN.

---

## Why Tests Cannot Be Fixed in Allowed Scope

The `killer_demo.rs` tests assert AC-1 through AC-7:
- **AC-1**: Bad fixture rejects with 2 code findings (FUNC_LOOPS_FOR + CLIPPY_UNWRAP_USED)  
  → Requires lanes to run and produce typed findings. Production bug #1 + #2.
- **AC-4**: Gate failures empty (all 7 lanes clean/findings)  
  → Requires lanes to run and succeed. Production bug #1 + #3.
- **AC-7**: Per-lane contains all 7 Edit lanes with clean outcomes  
  → Requires lanes to run. Production bug #1 + #3.

Test-only changes (e.g., having the test run lanes before check) would:
1. Bypass the `check` command's lane execution (which is the tested behavior)
2. Still fail on bug #2 (clippy produces wrong rule_id)
3. Still fail on bug #3 (dylint artifact parse error)

Redefining "green" to accept current all-gate-failures behavior violates Main's explicit instruction: "Do not weaken `killer_demo.rs` to accept current all-gate-failures behavior."

---

## Required Production Changes (Out of Scope)

| # | File | Change |
|---|------|--------|
| 1 | `crates/titania-check/src/main.rs:37` | Run all 7 lanes before `aggregate_scope` in the `check` command |
| 2 | `crates/titania-lanes/src/run_cargo_lane.rs:91-108` | Invoke `clippy_normalizer::normalize_clippy_jsonl` for Clippy lane output |
| 3 | `crates/titania-lanes/src/run_cargo/outcome.rs:108` or artifact writer | Fix `LaneOutcome` serialization to match deserializer expectations |

---

## Conclusion

The `killer_demo.rs` tests cannot reach GREEN within the 5-file allowed scope. The test contract (AC-1 through AC-7) requires lane execution and typed clippy findings that the production binary does not currently support. Four independent production bugs block test completion.

---

## Final Resolution — 2026-07-05

This blocker is superseded by the closed dependency repairs `tn-vab`, `tn-dzp`, `tn-z3y`, `tn-b5j`, and `tn-zuv`.

Current acceptance evidence is recorded in `.beads/tn-pdn/evidence-bundle.md` and raw JSON under `.beads/tn-pdn/raw/`.

Resolution details:
- `titania-check --scope edit --emit json` now runs edit lanes before aggregation (`tn-z3y`).
- Clippy lane output is normalized to concrete `CLIPPY_*` rule IDs (`tn-dzp`).
- LaneOutcome artifact serialization/deserialization is consistent (`tn-vab`).
- Subdirectory target discovery and root aggregation are fixed (`tn-b5j`).
- Pass reports carry typed per-lane entries and validate lane identity/order (`tn-zuv`).

Environment reconciliation:
- The default shell PATH used in one diagnostic run did not include `/cache/cargo-shared/bin`, so `cargo-dylint` was reported unavailable and the repaired fixture rejected with a Dylint infra failure.
- `cargo test` runs with `CARGO_HOME=/cache/cargo-shared` and a PATH that includes `/cache/cargo-shared/bin`; `strace -f -e execve cargo test -p titania-check --test killer_demo repaired_fixture_passes_with_receipt -- --exact` showed the test executing `/home/lewis/src/titania/.worktrees/v1-combined-dispatch/target/debug/titania-check` and successfully spawning `/cache/cargo-shared/bin/cargo-dylint`.
- Direct CLI acceptance was rerun against fresh temp fixture copies with `/cache/cargo-shared/bin` prepended to PATH. Bad fixture exits 1 with exactly `CLIPPY_UNWRAP_USED` and `FUNC_LOOPS_FOR`, zero gate failures, and all seven edit lanes. Repaired fixture exits 0 with `schema_version=1` receipt and all seven edit lanes.

Contaminated diagnostic excluded:
- Running the bad fixture in place under the repo tree also sees the parent repository `clippy.toml` and emits `CLIPPY_DISALLOWED_METHODS`.
- That is not the independent fixture contract. The accepted evidence uses fresh temp fixture copies, matching `killer_demo.rs`, so parent policy does not leak into the demo fixture.

Final status: RESOLVED. `tn-pdn` can close.
