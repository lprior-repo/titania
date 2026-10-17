# Domain model — tn-c6n (receipt domain)

## Ubiquitous language

Lane name, lane digest, receipt period, recorded target root, receipt
envelope, schema version, digest bundle, lane exit, finding count.

## Value objects (all smart constructors; illegal states unrepresentable)

- `LaneName`: non-empty, NUL-free text. Invalid input never becomes a value:
  construction returns `Err`, it does not trap or default.
- `LaneDigest`: `(lane, exit, scanned, passed, finding_count)` with the
  invariant `passed <= scanned` checked at construction.
- `ReceiptPeriod`: `(started_at, finished_at)` with `finished_at >=
  started_at` checked at construction.
- `RecordedTargetRoot`: absolute, non-empty, NUL-free UTF-8 path. No
  filesystem I/O at read time (archived receipts outlive the judged project).
- `ReceiptEnvelope`: schema-versioned assembly of target root, period, lane
  digests, and the four-digest bundle. Only schema 2 assembles.
- `ReceiptDigests`: total constructor over four already-validated digests.

## Commands / outcomes

Construct (`new`), load from JSON text, render to JSON text. Every fallible
step returns a typed `ReceiptError`; rejection is observable, never silent.

## Policies (from tn-6r36, applied here)

- Boundedness by construction: rejection rules are total predicates over the
  inputs, no loops carry proof weight.
- Approved recovery: none needed — constructors have no failure recovery
  paths; errors propagate to the caller unchanged.
- Unproven-required blocks: formal lanes that cannot execute here are
  recorded as residuals, never as passes.

## Open domain questions

None. All six retained requirements map to existing constructors.
