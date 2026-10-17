# Hazard analysis — tn-4isu

- H1 Vacuous pass via short unwind: MITIGATED by construction — Kani checks
  unwinding assertions by default; insufficient bounds FAIL loudly. Every
  SUCCESSFUL claim requires the raw log showing unwinding checks enabled
  (no `--no-unwinding-checks` flag anywhere).
- H2 Property weakening disguised as bound change: MITIGATED by review —
  the diff must show ONLY `#[kani::unwind(N)]` lines changed. Any assert/
  assume/cover/input edit rejects the bead.
- H3 Substituting a smaller behavior (fewer inputs, shorter arrays):
  FORBIDDEN (INV-1). `KANI_ID_CONTRACT_BOUND = 32` and `KANI_DIFF_CANDIDATE_BOUND
  = 8` stay fixed.
- H4 Blind resource-cap lift: FORBIDDEN (D6). Same `-j 1`, same cgroup, same
  600s lane budget.
- H5 Hallucinated Kani output: MITIGATED — every verdict cites a raw log file
  under `evidence/` with command, exit status, tool version.
- H6 Machine exhaustion during triage: MITIGATED — cgroup caps + per-harness
  `timeout(1)` wrappers + lightest-first order.
- Residual: CBMC version-specific performance; a bound sufficient under
  cargo-kani 0.67.0 is recorded with that exact version, not generalized.
