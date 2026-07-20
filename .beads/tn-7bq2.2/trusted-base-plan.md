# Trusted Base Plan

> Trust markers the v1.5 lanes will introduce, mapped to handler.
> Every `assume`, `axiom`, `admit`, `external_body`, `trusted`, `ignore`,
> stub, disabled check, model bound, or model reduction needs a
> `trusted-base-ledger/v1` row.

## Trust markers

| Marker | Location | Justification | Compensating evidence |
|--------|----------|---------------|-----------------------|
| `cfg(kani)` indirection around harness body | `crates/titania-core/src/kani.rs` | Required by Kani to call kani:: functions under `cfg(kani)`. | Proptest shadow on production code; CI runs both. |
| cgroup wrapping via `systemd-run --user --scope` | lane shell | Trusts that systemd correctly caps memory; Kani would otherwise OOM the box. | Per-harness run; lane reports `PROOF_KANI_BLOCKED` on cgroup exit; never silent failure. |
| `cargo mutants` exit-status trust | lane shell | Trusts cargo-mutants' classification of `success`/`caught`/`timeout`/`unviable`. | Fuzz covers parse logic on outputs. Schema-version check guards forward-compat. |
| `cargo mutants` baseline JSON read | `titania-core/src/mutants_baseline.rs::load` | Trusts serde_json round-trip; baseline file is JSON v1. | MutantsBaseline::load returns typed errors on parse/schema failures; never silent acceptance. |
| `cargo-kani --format json` inventory parser | `crates/titania-core/src/kani_inventory.rs` (new) | Trusts cargo-kani's listed JSON shape. | fuzz `parse_inventory` lane (v15.F1). |
| `cargo mutants` outcomes parser | `crates/titania-core/src/mutants_outcomes.rs` (new) | Trusts the cargo-mutants outcomes/mutants JSON schemas. | fuzz `parse_outcomes` lane (v15.F2). |
| `RUSTFLAGS=--cfg loom` indirection | `crates/titania-lanes/src/artifact_writer.rs` | Loom-required cfg indirection (skill rule). | Loom test runs under CFG; production builds unchanged. |

## Counter-trust markers

None. No `assume`, `axiom`, `admit`, `external_body`, `#[trusted]`,
`#[ignore]`, `#[kani::stub]` (without contract), or model reduction
flags appear in v1.5 proof artifacts.

## Expiry / Review

- All trust markers are recorded with `owner: rust-contract`, `scope:
  per-package`, `expiry: 2026-12-31` (a soft review horizon; not a
  re-validation deadline).
- `compensation: layered` — every trust marker has at least one
  non-trust verify path (fuzz / proptest / loom).
