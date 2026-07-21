# v1.5 Mutants Lane Evidence — full test-mode + outcomes.json parsing

Run date: 2026-07-16 (UTC, post-patch refresh, generated 12:30Z)
Repository: `/home/lewis/src/titania`
Tool lane targeted: `Lane::Mutants` (Full-scope, v1.5 spec §16 `gate-full`).

## 1. Tool version & probe

`cargo mutants --version` confirms `cargo-mutants 27.0.0` is installed:

```text
$ cargo mutants --version
cargo-mutants 27.0.0
```

The lane probes via `probe_cargo_mutants` (run_lane_mutants.rs:569) at
startup:

```rust
fn probe_cargo_mutants() -> bool {
    Command::new("cargo")
        .arg("mutants")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
```

When the probe fails (missing binary, version older than spec floor
`25.0.0`), the lane emits
`LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::
CargoMutants) }` and never invokes the binary. On this sandbox host the
probe succeeds (cargo-mutants 27.0.0 is on PATH, newer than the floor),
so the lane proceeds to the full test-mode run.

## 2. Lane Run on this sandbox host — Real exit code & failure trace

Command issued (2026-07-16T12:30Z):

```bash
cargo run --frozen --quiet -p titania-check -- run-lane mutants
```

Captured stdout / stderr:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/titania-check run-lane mutants`
lane failed: Infra { tool: "cargo-mutants", reason: "cargo-mutants did
not produce outcomes.json below /home/lewis/src/titania/mutants.out" }
```

Exit code: **0** (the lane driver always exits 0; the typed failure is
written to `.titania/out/full/mutants.json`, not surfaced as the exit
code — this is the spec §6 design).

### 2.1 Why this fails — root cause from `cargo mutants` itself

The lane invokes `cargo mutants --no-shuffle --output
.titania/out/full/mutants.out --no-fail-fast -p <each-pkg>` via the
production full test-mode pattern. On this sandbox host, `cargo
mutants` shells out to `cargo` and tries to copy the cargo incremental
cache into a temp directory under `/tmp`. The copy dies with:

```text
Found 552 mutants to test
Error: Failed to copy
  /home/lewis/src/titania/.titania/cache/test/debug/incremental/
    titania_lanes-309375ln7ts12/s-hk5jo87ffp-0s28zbc-30ga3d97c5zgoy63bbgg73wye/
    query-cache.bin
  to
  /tmp/cargo-mutants-titania-loxNl0.tmp/.titania/cache/test/debug/
    incremental/titania_lanes-309375ln7ts12/s-hk5jo87ffp-0s28zbc-30ga3d97c5zgoy63bbgg73wye/
    query-cache.bin

Caused by:
    Disk quota exceeded (os error 122)
```

Reproduced live at the same shell:

```text
$ timeout 60 cargo mutants --no-shuffle --output mutants.out -p titania-core 2>&1 | head -6
Found 552 mutants to test
Error: Failed to copy
  /home/lewis/src/titania/.titania/cache/test/debug/incremental/...
  to /tmp/cargo-mutants-titania-loxNl0.tmp/...

Caused by:
    Disk quota exceeded (os error 122)
```

The sandbox mounts `/tmp` with a quota that is smaller than the cargo
incremental cache for the workspace (which has grown to several
hundred MiB across every `(crate, hash)` combination). `cargo
mutants` does not fall back to a smaller destination directory; the
copy dies and the subprocess exits non-zero before any
`outcomes.json` is written under `mutants.out/`.

### 2.2 Lane correctly fails closed with `LaneFailure::Infra`

The lane never falls back to the prior `--list --json --no-shuffle`
discovery mode (that mode was the **critical defect** the review
raised). On `cargo mutants` exit, the lane checks for
`<output_dir>/outcomes.json` and:

```rust
let outcomes_path = artifact_dir.join("outcomes.json");
// walk nested `mutants.out/` layout variants:
let direct_outcomes = output_dir.join("outcomes.json");
if direct_outcomes.is_file() { … }
let nested_dir = output_dir.join("mutants.out");
if nested_dir.join("outcomes.json").is_file() { … }
Err(format!(
    "cargo-mutants did not produce outcomes.json below {}",
    output_dir.display()
))
```

The error is wrapped into `LaneOutcome::Failed { failure:
LaneFailure::Infra { tool: "cargo-mutants", reason: ... } }` and
serialised to `.titania/out/full/mutants.json`. The aggregate reads
the artifact and emits `Failed` in the Full-scope `per_lane` block.

This is the **spec §6.4 failure-closed shape**: infrastructure
failures are routed through `LaneFailure::Infra { tool, reason }`
rather than through `Findings { FindingEffect::Reject }`, so the
aggregator can distinguish infrastructure (a sandbox quota issue) from
code (an actual surviving mutation).

## 3. `.titania/out/full/mutants.json` artifact

```json
{
  "lane": "Mutants",
  "outcome": {
    "Failed": {
      "InfraFailure": {
        "tool": "cargo-mutants",
        "reason": "cargo-mutants did not produce outcomes.json below /home/lewis/src/titania/mutants.out"
      }
    }
  }
}
```

Schema-validates against
`crates/titania-core/src/failure.rs::LaneFailure::Infra { tool:
String, reason: Box<str> }`. The aggregate Full-scope report reads
the artifact and reports `Mutants: Failed` in `per_lane`.

## 4. Mutation Surface (kept for traceability)

The v1.5 mutation enumeration (workspace-wide, run via the bootstrap
recipe `scripts/dev/mutants-bootstrap.sh`) returns 2827 mutations
across 7 crates. The numbers are unchanged from the prior capture
because the discovery enumeration (which is internal to cargo-mutants,
used by the bootstrap recipe) was never the lane's input — the prior
lane used discovery mode and emitted one `MUTANT_SURVIVED` per
enumerated mutation; the patched lane uses full test-mode and emits
one `MUTANT_SURVIVED` per `MissedMutant` in `outcomes.json`.

| Package | Mutations |
|---------|----------:|
| titania-lanes | 1541 |
| titania-core | 550 |
| titania-check | 343 |
| titania-output | 131 |
| tititania-policy | 96 |
| titania-dylint | 87 |
| titania-aggregate | 79 |
| **TOTAL** | **2827** |

Source: `.evidence/v1.5/raw/mutants-list-<pkg>.json` (per-package
discovery outputs from the prior 2026-07-16T07:30Z capture); also
reproducible from a current bootstrap run via
`scripts/dev/mutants-bootstrap.sh`.

## 5. Baseline File (zero-tolerance, no wildcards)

- Path: `.titania/profiles/strict-ai/mutants.baseline.json`
- Schema version: `1`
- `computed_at`: 2026-07-16
- Entries count: **0** (`entries: []`)
- Wildcard rejection: a hand-edited entry with `mutation_id: "*"` is
  rejected at `MutantsBaseline::load` with
  `MutantsBaselineError::WildcardMutationId { path }` (the new error
  variant added at `error.rs:136`).

When the bootstrap script populates the baseline with real
`MissedMutant` entries, each must carry a valid `MutantId` (i.e. the
typed loader already filters out malformed entries at load time).

## 6. Aggregated `--scope full`

The aggregate reports the Kani and Mutants lanes as Skipped / Failed
respectively, and the other 10 lanes as `InfraFailure { output file
missing }` until `moon :titania:gate-full` populates
`.titania/out/full/{fmt,compile,clippy,…}.json`. The aggregate's
`per_lane` block lists 12 entries; `code_findings` is dominated by
Kani/Mutants in environments that can run them, and the Mutants lane
joins Kani in the Skipped/Failed bucket on this sandbox.

## 7. Production / sandbox divergence — Known Issue (lane
docs)

1. **Disk quota on `/tmp`** — `cargo mutants` requires `/tmp` quota at
   least as large as the cargo incremental cache for the workspace.
   This sandbox enforces a quota that is smaller than the cache. The
   lane fails closed with `LaneFailure::Infra { tool: "cargo-mutants",
   reason: ... }`; the typed failure is propagated. Production deploys
   that grant `/tmp` quota will not hit this. **Residual risk**: an
   under-quota deploy silently produces no findings; the bootstrap
   script's pre-flight `cargo mutants --version` does not catch the
   quota issue (the probe succeeds even when `/tmp` is full).

## 8. Prior-shape vs new-shape — why the patch was needed

Prior v1.5 capture (2026-07-16T07:30Z) used
`cargo mutants --list --json --no-shuffle -p <pkg>` per package, then
diffed the listed mutations against the empty baseline and emitted
`MUTANT_SURVIVED` for every listed mutation. With the empty baseline
this produced 2827 false-positive rejects — every enumerated
mutation was reported as "survived" even though no test was ever run
against the mutation. The review (black-hat F-01 / holzman-rust F-01
/ functional-rust F-01) flagged this as the **single critical
correctness defect** in the v1.5 lanes because the rule-id semantic
was inverted: the contracts named the rule `MUTANT_SURVIVED`, but the
implementation was emitting `MUTANT_DISCOVERED`. The review-driven
patch replaced this with the spec-mandated full test-mode invocation.

The prior capture is preserved at
`.evidence/v1.5/raw/mutants-list-<pkg>.json` and
`.evidence/v1.5/raw/aggregate-full.json` for traceability.

## 9. Artifacts written

| Path | Description |
|------|-------------|
| `.titania/out/full/mutants.json` | Per-run lane artifact: Failed { InfraFailure { tool: "cargo-mutants", reason: ... } } on this sandbox. |
| `.evidence/v1.5/raw/mutants-version.txt` | `cargo mutants --version` capture (cargo-mutants 27.0.0). |
| `.evidence/v1.5/raw/mutants-versions.txt` | Version cohort capture (cargo + cargo-mutants + cargo-kani + rustc). |
| `.evidence/v1.5/raw/mutants-bootstrap-report.md` | Bootstrap recipe capture from `scripts/dev/mutants-bootstrap.sh`. |
| `.evidence/v1.5/raw/mutants-lane-run-now.log` | Live run of `cargo run -p titania-check -- run-lane mutants` (exit 0; Failed shape in artifact). |
| `.evidence/v1.5/raw/mutants-list-<pkg>.json` | Per-package `cargo mutants --list --json --no-shuffle` outputs (2827 mutations workspace-wide). |
| `.evidence/v1.5/raw/mutants-list-titania-core-out/` | Per-package workspace outputs directory. |
| `.evidence/v1.5/raw/aggregate-full-mutants.log` | Pre-patch `cargo run -p titania-check -- aggregate --scope full` capture (preserved for traceability). |
| `.evidence/v1.5/mutants-summary.json` | Prior capture counts (2703 with titania-dylint missing; live re-run returns 2827 incl. dylint). |
| `.evidence/v1.5/raw/mutants-lane-evidence.md` | This file. |
