# Domain model — tn-vr2n

Ubiquitous language: effective invocation (full cargo argv the runner
spawns), mandatory lint severity (the `-D` denies in `CLIPPY_ARGS`),
lint ceiling (`--cap-lints`, a rustc flag that caps every lint level and
therefore neuters `-D` denies wherever it appears), chokepoint
(`run_cargo_lane::run_lane`, the single join of lane argv + extra args +
spawn), trusted receipt (`LaneOutcome::Clean` with command evidence).

Entities: `CargoLane` (closed enum of five lanes), `CommandIn` (sole spawn
builder), `LaneReport`/`Finding` (scanner verdicts), `LaneOutcome`
(Clean/Findings/Failed).

Value objects: validated extra-args slice (every element screened for a
lint-ceiling token); static lane argv (pinned by exact-match tests).

The bead preserves one invariant: no effective invocation judged by the
protected runner carries a lint ceiling.
