# tn-8qfc State

Bead: must-use/future floor (P0, in_progress). Single-operator in-checkout.

- Contract: DONE (contract.md, 3 proof seeds, traceability 3/3).
- Implementation: DONE. Scanner floor 16→20 entries; root + template pin
  `clippy::let_underscore_must_use`; dylint `REQUIRED_LINTS` parity.
  Futures covered through the same floor (`Future: #[must_use]`).
- Tests: DONE. 2 new lane tests (weakening matrix + deny-clean);
  canonical count 16→20 (contract-driven, per-entry assertions kept);
  1 new dylint fixture test. Suites: 21/21 lanes, 1/1 new dylint.
- Gates: fmt exit 0; workspace check exit 0; strict source clippy exit 0;
  rustdoc -D warnings exit 0; lanes+dylint suites 401 pass / 0 fail
  (incl. one regression found+repaired: exceptions fixture floor pins);
  final `moon ci` running.
