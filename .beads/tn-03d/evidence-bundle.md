# Evidence Bundle — tn-03d (v1 Domain Model)

## Requirements → Evidence Map

| Requirement | Spec Ref | Source | Test | Raw Command |
|---|---|---|---|---|
| Lane enum (10 variants) | contract.md §1 | lane.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| GateScope enum (3 variants) | contract.md §1 | gate_scope.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| Report::pass() with per_lane>=1 | contract.md §3 | report.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| Report::reject() empty collections error | contract.md §3 | report.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| Finding smart constructor | contract.md §3 | finding.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| Location span validation | contract.md §3 | finding.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| RepairHint::patch zero-width error | contract.md §3 | finding.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| LaneOutcome::Clean exit=0 validation | contract.md §3 | outcome.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| LaneEvidence new() | contract.md §3 | outcome.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| CommandEvidence argv[0]==executable | contract.md §3 | outcome.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| ProcessTermination::Signaled rejects on Windows | contract.md §3 | outcome.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| QualityReceipt schema_version validation | contract.md §3 | v1_receipt.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |
| serde round-trip all 19 types | contract.md §3 | tests/tn_03d_domain_model.rs | tn_03d_domain_model | `cargo test -p titania-core --test tn_03d_domain_model` |

## Raw Evidence

### Tests (60 passed, 0 failed)
```
cargo test -p titania-core --test tn_03d_domain_model
```
Exit: 0
60 tests, 1 suite, 0 failures.
Raw: `.beads/tn-03d/raw/tn_03d_tests.txt`

### Cargo fmt (pass)
```
cargo fmt --all -- --check
```
Exit: 0
Raw: `.beads/tn-03d/raw/cargo_fmt.txt`

### Clippy (pass, strict source gate)
```
cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings
```
Exit: 0
Raw: `.beads/tn-03d/raw/clippy.txt`

### Cargo vet (pass)
```
cargo vet
```
Exit: 0
34 fully audited, 1 partially, 48 exempted.
Raw: `.beads/tn-03d/raw/cargo_vet.txt`

## Type Summary (19 types)

1. `Lane` — 10 variants (unit enum)
2. `GateScope` — 3 variants + #[non_exhaustive]
3. `SkipReason` — 4 variants
4. `Report` — Pass/Reject/PolicyError/InputError
5. `RejectKind` — CodeOnly/GateOnly/Mixed
6. `Finding` — struct with 6 fields
7. `FindingEffect` — Reject/Informational
8. `Location` — Span/Dependency/Manifest/Workspace/Tool
9. `RepairHint` — 7 variants
10. `LaneOutcome` — Clean/Findings/Failed/Skipped
11. `LaneEvidence` — struct with 4 fields
12. `CommandEvidence` — struct with 2 fields
13. `LaneFailure` — 4 variants
14. `ProcessTermination` — 5 variants
15. `QualityReceipt` — struct with 7 fields + schema_version
16. `LaneReceipt` — struct with 3 fields
17. `PolicyDiagnostic` — struct with 3 fields
18. `InputDiagnostic` — struct with 3 fields
19. `DiagnosticSeverity` — Error/Warning

## Invariants Verified

- `Lane` is unit enum, 10 variants, FromStr PascalCase
- `GateScope` is #[non_exhaustive], 3 variants
- `Report::pass()` validates per_lane.len() >= 1
- `Report::reject()` rejects empty collections
- `LaneOutcome::Clean()` rejects non-zero exit
- `RepairHint::patch()` rejects zero-width ranges
- `CommandEvidence::new()` validates argv[0] == executable
- `ProcessTermination::Signaled()` rejects on Windows
- All 19 types serde round-trip tested
- No unwrap/expect/panic in production code
