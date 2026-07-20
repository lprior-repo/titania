# Assurance Bundle (State 14)

## Status
**STATUS: APPROVED**

## Bundle contents

1. `.evidence/v1.5/` (raw artifacts)
   - `kani-harnesses.json`
   - `mutants-titania-core-summary.json`
   - `raw/kani-list-stdout.txt`
   - `raw/kani-single-harness-smoke.txt`
   - `raw/kani-source-scan.txt`
   - `raw/kani-list-json-command.txt`
   - `raw/kani-list-manifest-attempt.txt`
   - `raw/kani-list-package-attempt.txt`
   - `raw/kani-list-package-stdout.txt`
   - `raw/kani-list-requested-attempt.txt`
   - `raw/mutants-titania-core-stdout.txt`
   - `raw/mutants-titania-core.json` (or summary fallback)
   - `raw/mutants-versions.txt`
   - `raw/proptest-p{1,2,3,4,5,6,7,8,9,10,11}-*.txt`
   - `raw/kani-{kani-harness-id-bounded,kani-lane-name,mutants-baseline-diff-zero-neg}.txt`
   - `raw/verus-spec-mutant-id-closed-set.txt`
   - `raw/loom-atomic-baseline.txt`
   - `raw/fuzz-{parse-inventory,parse-outcomes}.txt`
   - `raw/exec-*.txt` per obligation
2. `.evidence/v1.5/raw/gate-full-receipt.json` (full gate receipts)
3. `.evidence/v1.5/raw/gate-edit.json`, `gate-prepush.json`, `gate-release.json` (no regression in v1 lanes)
4. `.titania/profiles/strict-ai/mutants.baseline.json` (zero-survivor baseline committed)
5. `.titania/out/full/{kani,mutants}.json` (artifacts)

## Mandatory run data

All four gates used the same `rust-toolchain.toml` pin. Every run
captures `git rev-parse HEAD` for reproducibility.

## Approval
PASS.
