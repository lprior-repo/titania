# tn-pdn — Workflow Model

## Report Lifecycle State Machine

```
                    ┌─────────────────────────────────────────────┐
                    │                                             │
                    │  titania-check --scope <scope>              │
                    │         --emit json                         │
                    │         [--out <path>]                      │
                    │         <target>                            │
                    ▼                                             │
┌──────────┐    ┌─────────────┐    ┌───────────┐    ┌──────────────────┐
│ PolicyError│    │  InputError │    │  lanes    │    │  Pass { receipt, │
│  │        │    │   │         │    │  execute  │    │   per_lane }      │
│          │    │             │    │   │       │    │   │                │
└──────────┘    └─────────────┘    └─────┬───┘    └──────────────────┘
                                          │
                    ┌─────────────────────┘
                    │
                    ▼
            ┌──────────────────────┐
            │  assemble_report     │
            │  per_lane → Report   │
            └──────────┬───────────┘
                       │
              ┌────────┼──────────┐
              │        │          │
              ▼        ▼          ▼
       ┌────────────┐ ┌────────┐ ┌──────────┐
       │ Reject {   │ │ Reject │ │ Reject { │
       │  CodeOnly  │ │ Mixed  │ │ GateOnly │
       │  code_only }│ │both    │ │ gate_only}│
       └────────────┘ └────────┘ └──────────┘
```

### State Definitions

| State | Meaning | Entry Condition |
|-------|---------|-----------------|
| `PolicyError` | Policy config unparseable/unavailable | Policy file missing or invalid TOML |
| `InputError` | CLI argument validation failed | Unknown scope, invalid --emit value |
| `ExecutingLanes` | All scope lanes dispatched in order | Scope is valid, policy loads |
| `Pass` | All scope lanes produced Clean or Skipped | No rejecting findings, no failures |
| `Reject.CodeOnly` | Analysis lanes produced rejecting findings only | `code_findings` non-empty, `gate_failures` empty |
| `Reject.GateOnly` | Infrastructure failures only | `gate_failures` non-empty, `code_findings` empty |
| `Reject.Mixed` | Both findings and failures | Both collections non-empty |

### Guard Conditions

- **Scope guard**: `GateScope::from_str` must succeed. Unknown scope → `InputError`.
- **Policy guard**: Policy file must parse as valid TOML. Unparseable → `PolicyError`.
- **Compile dependency**: `Test`, `Dylint`, and dependent lanes skip if `Compile` outcome is `Failed`.
- **Reject invariant**: `assemble_report` checks `check_reject_not_empty` — a Reject with both empty is impossible.

### Transition Rules

1. **PolicyError → terminal**: No lanes run. Report emitted immediately with `PolicyError { diagnostics }`.
2. **InputError → terminal**: No lanes run. Report emitted immediately with `InputError { diagnostics }`.
3. **ExecutingLanes → Pass**: All lanes in scope are `Clean` or `Skipped`. Receipt assembled with schema_version=1.
4. **ExecutingLanes → Reject**: Any lane outcome is `Findings { findings }` with at least one `Reject` effect, or any `Failed`.
5. **ExecutingLanes → Pass (informational)**: All lanes produce `Informational` findings only. Lane passes; receipt emitted.

### Edit Scope Lane Order

`Edit` scope runs these 7 lanes in order (from `gate_scope.rs`):
1. `Fmt` — format check
2. `Compile` — cargo check
3. `Clippy` — cargo clippy
4. `AstGrep` — structural rules
5. `Dylint` — type-aware dylint
6. `PanicScan` — regex scan for assert!/panic!
7. `PolicyScan` — policy violation scan

`Compile` must succeed before `Dylint` and `Test` can run (dependency).

## Killer Demo Workflow

### Bad Fixture Path

```
titania-check --scope edit --emit json fixtures/strict_ai_loop_unwrap/bad
  → Fmt: Clean
  → Compile: Clean (for-loop compiles fine)
  → Clippy: Findings { CLIPPY_UNWRAP_USED }  ← repair: RequiresHumanReview
  → AstGrep: Findings { FUNC_LOOPS_FOR }     ← repair: UseIteratorPipeline
  → Dylint: Clean or Findings
  → PanicScan: Clean (no assert!/panic! macros)
  → PolicyScan: Clean
  → assemble_report → Report::Reject {
        code_findings: [CLIPPY_UNWRAP_USED, FUNC_LOOPS_FOR],
        gate_failures: [],
        per_lane: [all 7 outcomes]
    }
  → RejectKind::CodeOnly
  → Exit code: 1
```

**Key invariant**: Both findings in `code_findings`, ZERO in `gate_failures`. The bad fixture must NOT trigger infrastructure failures — that would make it `Mixed` instead of `CodeOnly`.

### Repaired Fixture Path

```
titania-check --scope edit --emit json fixtures/strict_ai_loop_unwrap/repaired
  → Fmt: Clean
  → Compile: Clean
  → Clippy: Clean (no unwrap, no clippy warnings)
  → AstGrep: Clean (no for loops)
  → Dylint: Clean
  → PanicScan: Clean
  → PolicyScan: Clean
  → assemble_report → Report::Pass {
        receipt: QualityReceiptV1 { schema_version: 1, ... },
        per_lane: [all 7 Clean outcomes]
    }
  → Exit code: 0
```

**Key invariant**: `schema_version = 1` in the receipt. All 4 digests present. All 7 lanes in `per_lane` with `LaneReceipt` summaries.

## Test Execution Workflow

```
cargo test -p titania-check --test killer_demo
  → spawn titania-check binary via CARGO_BIN_EXE_titania-check
  → create TempDir for bad fixture
  → write Cargo.toml + lib.rs (for-loop + unwrap)
  → run_in(tempdir, ["--scope", "edit", "--emit", "json"])
  → assert_report_reject(code_findings contains FUNC_LOOPS_FOR + CLIPPY_UNWRAP_USED)
  → assert_gate_failures_empty()
  → assert_reject_kind(CodeOnly)
  → create TempDir for repaired fixture
  → write Cargo.toml + lib.rs (iterator pipeline)
  → run_in(tempdir, ["--scope", "edit", "--emit", "json"])
  → assert_report_pass()
  → assert_receipt_schema_version(1)
  → assert_receipt_has_4_digests()
  → assert_per_lane_has_7_outcomes()
```
