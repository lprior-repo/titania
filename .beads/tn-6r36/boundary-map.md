# tn-6r36 Boundary Map

- **Pure core** — `ClassifySite`, `Bound::capped`, `RecoveryApproval::validate`,
  `ObligationState` transitions. Deterministic, no I/O, no time except an
  injected `today: PolicyDate` parameter (clock enters at the shell edge).
- **Imperative shell** — File reads (AST/HIR facts, `exceptions.toml`,
  profiles), Moon tasks (`moon ci`, lane tasks), Dylint driver invocation,
  report emission (`LaneReport`), metrics/logging.
- **Parser boundary** — External text (`exceptions.toml`, profile TOML,
  evidence refs) parsed once into trusted types; invalid input rejected with
  `ContractError`, never `panic`.
- **Async shell** — None in this bead. If later lanes add async execution,
  `async-rust-reviewer` owns spawn/cancellation/bounded-concurrency rules;
  core stays sync.
- **Unsafe/FFI** — None. Dylint ABI exception lives in `titania-dylint`
  only and is out of scope for this contract bead.
- **Current-enforcement note** — Today's `functional_surface.rs` rejects all
  `for`/`while`/`loop` and all `unwrap_or*` by syntax. This contract does not
  change that code; it defines the future classifier the follow-up beads
  (`tn-ebn9` bounded-execution, `tn-8obr` approved recovery, `tn-pfp1`
  resolved identities) will implement. No relaxation lands from this bead.
