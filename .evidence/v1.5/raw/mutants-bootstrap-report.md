# cargo-mutants v1.5 Bootstrap Report

Captured: 2026-07-16T02:54:42Z
Tool: cargo-mutants 27.0.0

## Tool Version

- `cargo mutants --version` → `cargo-mutants 27.0.0`
- Captured at `.evidence/v1.5/raw/mutants-version.txt`

## Per-Package Discovery (`cargo mutants --list --json --no-shuffle -p <pkg>`)

All six workspace crates returned exit status 0. No stderr was emitted on any
package (all `.stderr` files are 0 bytes).

| Package          | Exit | Mutation Count | stderr snippet |
|------------------|------|----------------|----------------|
| titania-core     | 0    | 549            | (empty)        |
| titania-lanes    | 0    | 1509           | (empty)        |
| titania-check    | 0    | 341            | (empty)        |
| titania-aggregate| 0    | 77             | (empty)        |
| titania-policy   | 0    | 96             | (empty)        |
| titania-output   | 0    | 131            | (empty)        |
| **Total**        |      | **2703**       |                |

Raw artifacts (one per package):

- `.evidence/v1.5/raw/mutants-list-titania-core.json`     (852,367 B)
- `.evidence/v1.5/raw/mutants-list-titania-lanes.json`    (2,497,440 B)
- `.evidence/v1.5/raw/mutants-list-titania-check.json`    (560,565 B)
- `.evidence/v1.5/raw/mutants-list-titania-aggregate.json`(134,736 B)
- `.evidence/v1.5/raw/mutants-list-titania-policy.json`   (152,289 B)
- `.evidence/v1.5/raw/mutants-list-titania-output.json`   (231,500 B)
- `.evidence/v1.5/raw/mutants-list-<pkg>.stderr`          (all 0 B)

## Build Check (`cargo mutants --check --no-shuffle -p titania-core`)

- Exit status: 0
- Raw artifact: `.evidence/v1.5/raw/mutants-check-titania-core.txt`
- Tail summary line: `549 mutants tested in 2m: 276 unviable, 273 succeeded`
- The unmutated baseline also passed (`ok  Unmutated baseline in 1s check`).

## Summary File

- `.evidence/v1.5/mutants-summary.json` — populated with all six package
  counts, the titania-core build-check result, and the tool version.

## Baseline Decision

The canonical empty baseline has been written to
`.titania/profiles/strict-ai/mutants.baseline.json`:

```json
{ "schema_version": 1, "computed_at": "2026-07-16T02:54:42Z", "entries": [] }
```

`entries: []` is correct because **no test-mode run has been performed yet**.
This bootstrap captures only the discovery surface (which mutants *could* be
generated) plus a `--check` of the build; it does **not** run the full
`cargo mutants` suite to classify each mutant as caught/missed/unviable, so
there are no entries to record.

## Bootstrap Script Status

- Expected bootstrap script: `scripts/dev/mutants-bootstrap.sh`
- **Status: MISSING** — the `scripts/` directory does not exist in the
  workspace root, so the bootstrap script is not present at all. A future
  bead should author `scripts/dev/mutants-bootstrap.sh` to encapsulate:
  1. `cargo mutants --list --json --no-shuffle -p <each crate>` (per-crate
     JSON capture, mirroring this bootstrap),
  2. `cargo mutants --check --no-shuffle -p titania-core` (build-check
     smoke),
  3. full `cargo mutants --no-shuffle -p <each crate>` test-mode runs to
     classify mutants and produce the real baseline entries,
  4. diff of the resulting `mutants.json` against
     `.titania/profiles/strict-ai/mutants.baseline.json` for regression
     detection,
  5. archival of raw artifacts under `.evidence/<version>/raw/`.

Until that script exists, all `cargo mutants` evidence collection must be
performed ad-hoc by hand exactly as done in this bootstrap.

## Residual Risk / Open Items

- **No test-mode classification run.** `--list` enumerates the mutation
  surface; it does **not** run tests against mutations. The real baseline
  cannot be populated until a test-mode run is executed. Any profile
  consumer of `mutants.baseline.json` that depends on `entries[]` should
  treat the empty list as "no classification performed yet" rather than
  "all mutants are caught."
- **Other workspace crates skipped at build-check.** Only `titania-core`
  was run through `--check`. The other five crates (`titania-lanes`,
  `titania-check`, `titania-aggregate`, `titania-policy`, `titania-output`)
  were enumerated but not build-checked. If the project requires a
  workspace-wide build-check before baseline adoption, that should be
  added as a follow-up bead.
- **Bootstrap script is not authored yet.** See "Bootstrap Script Status"
  above.
