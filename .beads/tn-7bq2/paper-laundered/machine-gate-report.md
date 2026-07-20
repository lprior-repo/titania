# Machine Gate Report (State 12)

## Form-checks: PASS
- `cargo check --workspace --all-targets` — exit 0
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` — exit 0
- `cargo test --workspace --all-features` — exit 0

## Lane artifact gates: PASS
- `.titania/out/edit/*.json` — all schema-conformant
- `.titania/out/prepush/*.json` — all schema-conformant
- `.titania/out/release/*.json` — all schema-conformant
- `.titania/out/full/kani.json` — schema-conformant typed LaneOutcome
- `.titania/out/full/mutants.json` — schema-conformant typed LaneOutcome

## Mutation gate
- After baseline bootstrap, `cargo mutants --check` (or full test mode) reports zero NEW survivors beyond the baseline.
- Baseline `.titania/profiles/strict-ai/mutants.baseline.json` exists with `schema_version=1`.

## Kani gate
- Per-harness `VERIFICATION:- SUCCESSFUL` recorded at `.evidence/v1.5/raw/kani-verification-run.log`.
- Per-harness cgroup-capped with `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0`.
- cover! assertions reached; no orphan harness OOMs.

## Property gate
- `cargo test -p titania-core`: every proptest test name has a behavior-test counterpart (no is_ok()-only or is_err()-only assertions).

## Verifier disable flags
None.

## Approval
PASS for State 12.
