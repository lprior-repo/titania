# Boundary map — tn-vr2n

Pure core (new): `policy_scan::invocation` — no I/O, no time, no env reads;
operates on a borrowed arg slice; deterministic.

Imperative shell (touched, one line + one variant): `run_cargo_lane::run_lane`
calls the pure guard before `cargo_output` spawns. No other shell changes.

Untouched boundaries: env readers, filesystem config reads, network
(registry), clocks, subprocess budgets — all reuse existing code.

No async, no threads, no FFI, no unsafe, no new dependencies.
