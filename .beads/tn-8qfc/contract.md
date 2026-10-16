# tn-8qfc Contract — Must-Use and Future Floor

Bead: `tn-8qfc` (P0, in_progress, owner Lewis). Scope: enforce Clippy/rustc
typed lints for discarded must-use results and forgotten futures
consistently across canonical policy, target manifests, Dylint weakening
protection, and the generated adoption template. Explicit handling/`await`
or approved ownership transfer (exceptions ledger) required. Covers
`let _ =` and bare statements. Early guard drop of non-`Result` bindings is
not `Result` handling (boundary shared with tn-6r36).

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| D1 | Canonical floor | `EXPECTED_LEVELS` gains deny `rust::unused_must_use`, `rust::unused_results`, `rust::let_underscore_drop`, `clippy::let_underscore_must_use` (16 → 20 entries). |
| D2 | Self-hosting | Workspace root and `titania/template` pin `clippy::let_underscore_must_use = deny` (rust trio already pinned). Members inherit via `workspace = true`, unchanged. |
| D3 | Dylint parity | `REQUIRED_LINTS` gains `clippy::let_underscore_must_use`; crate-level `allow` fires `BYPASS_REQUIRED_LINT_WEAKENING`. |
| D4 | Futures | Forgotten futures are rejected through the same floor: `Future` is `#[must_use]`, so discards trip `unused_must_use` and `let _ =` trips `let_underscore_must_use`. No separate future lint invented. |
| D5 | Approved transfer | `exceptions.toml` (owner + reason + expiry + review) remains the only approval path; no auto-approval, no new waiver kind. |
| D6 | No new lint families | Only the must-use floor; loop/method families belong to tn-ebn9/tn-pfp1. |

## Ubiquitous language (excerpt)

Must-use floor (deny minimum no manifest may go below); weakening (any
lower level); approved ownership transfer; forgotten future (unawaited,
unpolled `Future`).

## Open domain questions

None.

## Illegal states that remain representable

- A `let`-bound function pointer to a must-use value still evades
  expression-level detection (same documented limit as tn-pfp1).
- `policy.toml [lints]` overrides may still *lower* the floor by design
  (v1 §9.7); lowering is louder now (typed finding per entry) but possible.

## Proof seeds emitted

3 seeds: scanner weakening matrix, template/root self-hosting parity,
dylint weakening parity. Executable lanes: cargo unit tests + dylint
fixture tests.

## Next steps

Gates: fmt, workspace check, strict source clippy, lane + dylint suites,
`moon ci`. Then close with evidence.
