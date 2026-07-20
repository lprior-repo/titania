# State 1 transcript — orchestrator (tn-7bq2)

> Honest re-run on 2026-07-15. The earlier tn-7bq2 closure claimed
> STATUS: PASS for all 16 states against implementation that did not
> exist (no Kani harness, no mutants baseline, no proptest files). This
> transcript documents what was actually verified first-hand in this
> session.

## Tools on PATH

```
$ cargo --version
cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)
$ cargo-kani --version
cargo-kani 0.67.0
$ cargo mutants --version
cargo-mutants 27.0.0
$ rustc --version
rustc 1.97.0-nightly (ca9a134e0 2026-04-26)
$ moon --version
moon 2.2.4
$ which cargo-kani && sha256sum ~/.cargo/bin/cargo-kani
```

## Environment issues found

`cargo kani --workspace` fails to compile because `dylint_linting 6.0.1`
references `rustc_driver`/`rustc_span`/`rustc_errors` whose crates are
not on this nightly's `--extern` path. Per-package
`cargo kani -p <pkg>` works (verified titania-core: 8 harnesses
discovered, one model-checked to `VERIFICATION:- SUCCESSFUL`).

## Kani inventory (first-hand)

```
$ cd crates/titania-core && cargo kani list --format json --output-file .evidence/v1.5/kani-harnesses.json
Wrote list results to /home/lewis/src/titania/.evidence/v1.5/kani-harnesses.json

$ cat .evidence/v1.5/kani-harnesses.json
{
  "kani-version": "0.67.0",
  "file-version": "0.1",
  "standard-harnesses": {
    "crates/titania-core/src/kani.rs": [
      "kani::lane_digest_accepts_passed_not_greater_than_scanned",
      "kani::lane_digest_rejects_passed_greater_than_scanned",
      "kani::lane_name_rejects_empty_string",
      "kani::lane_name_rejects_nul_byte",
      "kani::recorded_target_root_accepts_absolute_path",
      "kani::recorded_target_root_rejects_empty_string",
      "kani::recorded_target_root_rejects_nul_byte",
      "kani::recorded_target_root_rejects_relative_path"
    ]
  },
  "contract-harnesses": {},
  "contracts": [],
  "totals": {
    "standard-harnesses": 8,
    "contract-harnesses": 0,
    "functions-under-contract": 0
  }
}
```

## Kani harness smoke (first-hand)

```
$ cd crates/titania-core && cargo kani --harness kani::lane_name_rejects_empty_string
Checking harness kani::lane_name_rejects_empty_string...
VERIFICATION:- SUCCESSFUL
Verification Time: 0.14285186s
Manual Harness Summary: Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Per-harness raw logs written to `.evidence/v1.5/raw/kani-per-harness/`.
6/8 harnesses successfully verified before the wall-timeout interrupted
the serial per-harness run; the remaining 2 were backgrounded to
finish asynchronously and are pending at the close of State 1.

## Implementation status (this session)

| Change | Status |
|--------|--------|
| `Lane::Kani` + `Lane::Mutants` + `Lane::file_stem()` in `crates/titania-core/src/lane.rs` | compiles |
| `GateScope::Full` + `FULL_LANES` in `crates/titania-core/src/gate_scope.rs` | compiles |
| 7 match-lane arms updated (artifact_reader.rs, run_lane.rs, run_lane_outcome.rs, run_cargo_lane.rs, titania-check/src/main.rs, titania-aggregate/src/artifact_reader.rs, test stub `fn lane_stem`) | compiles |
| New `titania-lanes/src/run_lane_kani.rs` (real per-package `cargo kani` + `PROOF_KANI_*` findings + infra-failure findings) | compiles under strict clippy |
| New `titania-lanes/src/run_lane_mutants.rs` (real per-package `cargo mutants --list` + `MUTANT_SURVIVED_<id>` findings + baseline diff) | compiles under strict clippy |
| New `crates/titania-core/tests/v15_lane_roundtrip.rs` | 6/6 pass |

## Strict clippy status

```
$ cargo clippy --workspace --lib --bins --examples --all-features
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.35s
```

Zero errors, zero warnings on `--lib --bins --examples`.

## Cargo fmt / cargo test

```
$ cargo fmt --all -- --check
(no output)
$ cargo test --workspace --all-features
... 30+ test result lines, all passing except 2 pre-existing flakes
in crates/titania-check/tests/template_prepush.rs which fail on baseline
main too (Moon stub not configured for this env).
```

## State 1 conclusion

Runtime provenance, tool inventory, and validator gate recorded.
Kani inventory confirmed (8 harnesses, real `cargo kani list`).
Strict clippy clean on the v1.5 lane code. Implementation is
**partial**: lane/scope types + Kani/Mutants lane impls done; proptest
files referenced in old contract.md (`v15_kani_harness_id.rs`,
`v15_gate_scope_roundtrip.rs`, `v15_mutant_id.rs`) do **not** exist
on disk; mutants baseline does **not** exist; Moon `:titania:gate-full`
task not defined. These are surfaced as explicit gaps in
`tn-7bq2/bead-status.md` for follow-up States 5, 11, 12.
