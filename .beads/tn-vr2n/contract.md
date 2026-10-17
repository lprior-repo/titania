# Contract — tn-vr2n

Source: bead tn-vr2n §§1–2 (EARS + KIRK), research-notes.md.

C1. Extra-arg ceiling rejected: any `--cap-lints` / `--cap-lints=*` element
in the runner's extra args is rejected with a typed domain error before any
child process starts; the lane exits Reject (Violations, exit 1) with the
offending token named. (Error path 1, CLI leg.)

C2. Static argv ceiling-free: the checked-in per-lane argv carries no
lint-ceiling token; a negative unit test pins this for all five lanes.
(Happy path 1: invocation evidence matches approved policy.)

C3. Existing channel locks (prove, don't duplicate): ceiling values in
`RUSTFLAGS` / `CARGO_ENCODED_RUSTFLAGS` → env findings; ceiling values in
`.cargo/config` rustflags → config findings; hostile parent `RUSTFLAGS`
dropped by the spawn scrub; surplus `run-lane` CLI tokens rejected by the
CLI parser. (Error paths 1–2, Happy path 2 by sccache-only wrapper rule,
unchanged.)

Out of scope: invocation-count metrics (forbidden by bead notes), new
scanner rules, Moon task edits, verifier lanes (parent exclusion).
