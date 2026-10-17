# Workflow model — tn-vr2n

Legal transitions for a cargo-lane run:

1. `run(lane)` → `run_checked` (lane name, cwd, target discovery).
2. `run_lane(target, lane, extra_args)` → NEW GUARD
   `validate_extra_args(extra_args)`; `Err(Policy)` short-circuits to
   `CargoRun::Error { Violations }` — no spawn, no outcome, no receipt.
3. `cargo_output` spawns the scrubbed subprocess (env already ceiling-free).
4. Outcome built (`Clean` only for exit 0 with recorded argv evidence).

Terminal states: `Clean` (argv recorded, exit 0), `Findings` (clippy
normalizer), `Failed`/`Error` (tool failure or policy block).

Guards: CLI parser rejects extra tokens; chokepoint rejects ceiling flags;
scrub drops ceiling-carrying env; scanners reject ceiling-carrying
config/env ahead of execution.
