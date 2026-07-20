# v1.5 Contract — Summary

> Bound to `.evidence/v1.5/spec.md`. This file is the contract deliverable
> index. It is consumed by the proof-writer (`tn-7bq2.2`), proof-to-implementation
> (`tn-7bq2.4` if created), test-writer, and implementation lanes.

## Status

- **Bead**: tn-7bq2.1 (P1, in_progress)
- **Spec**: `.evidence/v1.5/spec.md` (locked, .evidence/v1.5/)
- **Pre-impl evidence**:
  - `.evidence/v1.5/kani-harnesses.json` — 8 standard harnesses confirmed
  - `.evidence/v1.5/raw/kani-single-harness-smoke.txt` — `VERIFICATION:- SUCCESSFUL`
  - `.evidence/v1.5/raw/kani-list-stdout.txt` — cargo kani list exit 2 (rejected flags)
  - `.evidence/v1.5/raw/mutants-titania-core-stdout.txt` — 480 mutants / 236 build-survivors
  - bd memories: `v15-kani-inventory`, `v15-mutants-titania-core`

## Decisions (D1–D8)

| # | Decision | Choice |
|---|----------|--------|
| D1 | Lane/GateScope exhaustiveness | Total. 9 production match sites updated. |
| D2 | Scope placement | Kani/Mutants only in `GateScope::Full`. |
| D3 | Mutants baseline posture | Zero-survivor under full test-mode. |
| D4 | Kani scaling | Per-package; cgroup `-j 1`; CBMC OOM fails per-harness only. |
| D5 | Resource governance | Mandatory. No full-workspace Kani without waiver. |
| D6 | Proof artifacts | Per-finding `PROOF_KANI_*` and `MUTANT_SURVIVED`. |
| D7 | Baseline location | `.titania/profiles/strict-ai/mutants.baseline.json`. |
| D8 | Moon task names | `titania-kani`, `titania-mutants`, `:titania:gate-full`. |

## Artifacts

| Artifact | Path |
|----------|------|
| Domain model | `domain-model.md` |
| Type contracts | `type-contracts.md` |
| Workflow model | `workflow-model.md` |
| Error taxonomy | `error-taxonomy.md` |
| Boundary map | `boundary-map.md` |
| Hazard analysis | `hazard-analysis.md` |
| Proof seeds | `proof-seeds.jsonl` |
| Traceability | `traceability-matrix.jsonl` |

## Ubiquitous language (excerpt)

- **Kani harness** — A `#[kani::proof]` function.
- **Kani harness id** — `PROOF_KANI_<NAME>`, derived from harness name.
- **Mutant** — One cargo-mutants mutation; has stable `MutantId`.
- **Test-survivor** — A mutant that survives `cargo mutants` (NOT `--check`).
- **Mutants baseline** — JSON artifact at `.titania/profiles/strict-ai/mutants.baseline.json`.
- **Full gate** — `:titania:gate-full` Moon composite.

## Open domain questions

None. All decisions locked before contract emitted.

## Illegal states that remain representable

- `MutantId::new` with a non-canonical operator could now panic-free return a
  `MutantIdError::UnknownOperator`. The contract maps this to the
  `unknown_operator` rejection; the lane considers it a baseline corruption
  signal.
- `MutantsBaseline::load` with a missing file surfaces `MissingBaseline` not
  `panic`. The lane fires `MUTANT_BASELINE_MISSING` rather than crashing.

## Proof seeds emitted

17 seeds total: 11 proptest, 2 Kani, 1 Verus, 1 Loom, 2 fuzz.
See `proof-seeds.jsonl` for full text.

## Hazard highlights

- H1: CBMC OOM — cgroup cap `-j 1`, fail per-harness only.
- H3: Bootstrap scope — 236 build-survivors; full test-mode reduces, but
  bootstrap is operator work.
- H5: 9 production match-site updates; `cargo check --workspace --all-targets`
  catches every missed arm.
- H6: rustc_driver collision with titania-dylint; per-crate Kani + dylint-last.

## Next steps

- Close `tn-7bq2.1` with this contract.
- Open `tn-7bq2.2` proof-writer work; the seeds above feed the proof plan.
- Open `tn-7bq2.5` test plan/writer/reviewer; coverage expectations in
  proof-seeds.
- Open `tn-7bq2.3` (Kani lane impl), `tn-7bq2.4` (mutants lane + baseline
  bootstrap), `tn-7bq2.6` (gate-full + Moon tasks).
- Track evidence under `.evidence/v1.5/raw/` for kani-list, kani-runs,
  mutants-runs, gate-full receipts.

## Verification before close

- `cargo check --workspace --all-targets` green
- `cargo clippy --workspace --all-targets -- -D warnings` green
- `cargo test --workspace` green (no new failures)
- `cargo doc --workspace --no-deps -D warnings` green
- bd memory: `v15-contract-emitted` saved

## Sign-off line

Decision: contract ready; close `tn-7bq2.1`. Hand off to proof-writer
(`tn-7bq2.2`).
