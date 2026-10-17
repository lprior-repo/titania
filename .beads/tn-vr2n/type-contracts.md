# Type contracts — tn-vr2n

New public API in `titania_lanes::policy_scan::invocation`:

- `validate_extra_args(extra_args: &[String]) -> Result<(), InvocationError>`
  pure total function; `Ok` iff no element is a lint-ceiling token.
- `InvocationError::CapLints { arg: String }` — thiserror, carries the
  offending token; `result_large_err`/`result_unit_err` clean (small struct).
- Predicate `is_cap_lints_flag(arg: &str) -> bool` (private):
  `arg == "--cap-lints" || arg.starts_with("--cap-lints=")`.
  Case-sensitive (rustc flags are exact lowercase); no allocation on the
  accept path beyond iteration.

New error variant `RunCargoError::Policy(String)` in the (private)
`run_cargo` module; mapped to `LaneExit::Violations` (Reject, exit 1).

Illegal states unrepresentable: the chokepoint returns before spawn, so a
`CommandOutput`/`Clean` receipt cannot exist for a ceiling-carrying argv.
Static argv stays pinned by exact-match + negative unit tests.
