# Trusted-base plan — tn-4isu

Trust markers in scope (each gets a `trusted-base-ledger.jsonl` row at State 5):

- T1: unwind 16 on the diff harness (model bound; compensating evidence =
  SUCCESSFUL with unwinding assertions enabled, else revert).
- T2: unwind 24 on the wire harness (model bound; same compensation).
- T3: unwind 100 retained on the id harness (existing bound, diagnose-only).
- T4: 600s per-package lane budget (existing lane constant, unchanged).
- `kani::assume` input shaping in harnesses 3/4/9/11/12: pre-existing,
  non-vacuity covered by the harnesses' own `kani::cover!` points (unchanged).

No `axiom/admit/external_body/trusted/ignore/stub/disabled-check` markers are
introduced. No `--no-unwinding-checks` or other weakening flags at any point.
