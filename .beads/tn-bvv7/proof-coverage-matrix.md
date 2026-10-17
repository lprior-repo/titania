# Proof Coverage Matrix — tn-bvv7

| Obligation | Seed | Verifier | Executable evidence | Ledger |
|------------|------|----------|---------------------|--------|
| O1 | SEED-S1 | proptest | `tn_bvv7_fq_bypass_property` (64 cases) + exact FQ/alias/macro detector tests + catalog + lane tests | PASS |
| O2 | SEED-S2 | proptest | `tn_bvv7_effect_boundary_property` (64 cases) + exact process/async/state/scope tests + lane tests | PASS |
| R1 (bridge O1) | SEED-S1 | proptest | behavior tests `ast_grep_architecture` + `ast_grep_lane` (independent) + harness `tn_bvv7_fq_bypass_property` | PASS |
| R2 (bridge O2) | SEED-S2 | proptest | behavior tests `ast_grep_architecture` + `ast_grep_lane` + manifest test + harness `tn_bvv7_effect_boundary_property` | PASS |

Happy-path coverage: allowed fixtures, scope out-of-scope tests, lane
clean-controls test, manifest test. No obligation relies on listings,
coverage figures, or harness reachability alone.
