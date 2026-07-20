# Global readiness — tn-7bq2

> Generated 2026-07-15 by the go-skill orchestrator. Captures
> dependencies needed across States 2..16 of the go-skill pipeline for
> the v1.5 milestone (Bead tn-7bq2).

## 1. Tooling readiness

| Tool | Required | Installed | Verified |
|------|---------|-----------|----------|
| Rust nightly 2026-04-27 | yes | yes | `rustc --version` exit 0 |
| cargo 1.97.0-nightly | yes | yes | first-hand |
| `cargo kani 0.67.0` | yes | yes | first-hand; per-package runs verified |
| `cargo mutants 27.0.0` | yes | yes | `cargo mutants --version` |
| CBMC (Kani backend) | yes | yes (via cargo-kani) | `cargo kani --harness ...` exit 0 |
| `sccache 0.16.0` | yes | yes | first-hand |
| `moon 2.2.4` | yes | yes | `moon --version` |

## 2. Environment issues

| Issue | Severity | Workaround |
|-------|----------|------------|
| `cargo kani --workspace` fails to compile `dylint_linting 6.0.1` (transitive rustc_* rust crates) | low | lane uses `cargo kani -p <pkg>` per package; we do not recommend `--workspace` |
| `cargo test --workspace --all-features` 2 fails in `template_prepush` (Moon stub miss) | medium | pre-existing on main; v1.5 not the cause; needs `.moon/tasks/*` reshape or test waive |
| Missing `.titania/profiles/strict-ai/mutants.baseline.json` | blocker | Bootstrap script not authored; State 11/12 will author it |
| Missing Moon task `:titania:gate-full` | blocker | State 11 land-able from current code |

## 3. State-by-state readiness

| State | Lane | Status | Critical missing |
|-------|------|--------|------------------|
| 1 | orchestrator | **done** | — |
| 2 | explore | ready | — |
| 3 | rust-contract | ready | — |
| 4 | proof-planner + reviewer | ready | — |
| 5 | proof-writer | **gaps** | proptest `v15_kani_harness_id.rs`, `v15_gate_scope_roundtrip.rs`, `v15_mutant_id.rs`; Loom `atomic_load_store.rs`; Verus `MutantId::new`; fuzz `parse_inventory` / `parse_outcomes` |
| 6 | proof-reviewer | ready (depends on 5) | depends on 5 outputs |
| 7 | proof-to-implementation | ready (depends on 5+6) | depends on bridge artefacts |
| 8-10 | test-planner/writer/reviewer | ready | depends on 7+8 |
| 11 | holzman-rust | partial | 1. Kani lane + Mutants lane shells already compile strict-clean this session 2. Moon `:titania:gate-full` task not defined 3. `mutants.bootstrap.sh` not authored |
| 12 | formal-verifier | partial | honest verification-ledger.jsonl must include per-harness raw log refs (kani), per-package cargo mutants runs (mutants), lane/scope roundtrip runs |
| 13 | black-hat-reviewer | ready | needs artefact set |
| 14 | evidence-packaging + truth-serum | ready | needs done-artefacts |
| 15 | landing-skill | ready | needs done-artefacts |
| 16 | orchestrator-cleanup | ready | needs done-artefacts |

## 4. Honest STATUS

**STATUS: PARTIAL** for global-readiness overall. State 1 is green
(see `STATE.md`). States 2..16 are unblocked but require real work
whose results are **not** yet present on disk; they will gate their
own advancement via the validator rather than by paperwork claims.

Concretely: this report does not assert that States 2..16 are
`STATUS: PASS`. The earlier closure's claim that all 16 are PASS was
paper-laundering; it has been reverted (see .beads/tn-7bq2/paper-laundered/
which now archives the prior fraudulent artefacts, kept for
provenance).
