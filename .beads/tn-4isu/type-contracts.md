# Type contracts — tn-4isu

No production types change. Bounds relevant to unwind sufficiency:

- `KaniHarnessId::new(&str)`: loops = one `try_for_each` byte scan over
  `s.as_bytes()[1..]` (see `proof_id.rs:114`). Max trip: 96 (global cap), 32 for
  harness-9 generated inputs, 97-byte fixed overflow input ⇒ 96 body iterations.
  Therefore harness-9 `unwind(100)` covers max trip 96 with margin 4 — tight,
  not reducible without dropping the overflow case (forbidden by INV-1).
- `MutantsBaseline::contains/diff`: loops over ≤ 8 entries (see
  `mutants_baseline.rs:117-128`). Max trip 8 ⇒ `unwind(16)` keeps 2× margin.
- `LaneOutcome` serde round-trip over ≤ 8 input bytes: loop trips bounded by
  input length (8) plus serde_json internal constant-bound state machines;
  `unwind(24)` candidate, sufficiency proven iff Kani reports SUCCESSFUL with
  unwinding assertions enabled (default).
- `bounded_kani_lane_name` / `Lane::from_str`: short-string maps, no loops over
  symbolic data; `unwind(32)` retained.
