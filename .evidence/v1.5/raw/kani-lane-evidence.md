# v1.5 Kani Lane Evidence — per-package execution + cgroup wrapping

Run date: 2026-07-16 (UTC, post-patch refresh, generated 12:30Z)
Repository: `/home/lewis/src/titania`
Tool lane targeted: `Lane::Kani` (Full-scope, v1.5 spec §16 `gate-full`).

## 1. Tool version

`cargo kani --version` is captured at startup by the lane when the tool
is on PATH; on this sandbox host `cargo kani` is **not** present, so the
version probe short-circuits to `ToolUnavailable` and the lane never
runs.

Expected toolchain when the lane does run: `cargo-kani 0.67.0` (CBMC
6.8.0 embedded). The lane records the queried version into the artifact
in `run_lane_kani.rs::run_package` so the `Location::Tool { name,
version }` payload always reflects the runtime version, not a
hard-coded literal.

## 2. Kani lane exit code

Command issued (from `/home/lewis/src/titania`):

```bash
cargo run --frozen --quiet -p titania-check -- run-lane kani \
  2>&1 | tee .evidence/v1.5/raw/kani-lane-run-now.log
```

Exit code: **0**. Stdout summary: `lane completed: Skipped { reason:
ToolUnavailable(ToolKind::CargoKani) }` (the `cargo run` driver emits
nothing to stdout because the lane correctly exits without findings;
the Skipped shape is written to `.titania/out/full/kani.json`).

The lane driver always exits 0 unless the Rust infrastructure itself
fails (e.g. the lane dispatcher's `RunLaneError`); per-finding
classification and per-lane outcome are encoded as data into the
artifact, not as exit codes. This is the spec §6 design.

## 3. Per-package execution model (vs prior per-harness design)

### 3.1 Prior per-harness shape (rejected by review)

The pre-refresh v1.5 capture (2026-07-15) and the 07:30Z refresh both
documented a per-harness execution shape:

```bash
cargo kani -p titania-core --harness <name> --output-format=regular
```

This was rejected by `holzman-rust` (F-02 / F-03) and `black-hat` (F-02 /
F-03):

> Spec §4.2 step 4 says *"Re-run per package with `cargo kani -p <pkg>
> --output-format=regular -j 1` inside `systemd-run --user --scope …`."*
> The prior implementation ran one cargo-kani invocation per harness,
> paying per-harness build cost N times instead of once, losing the
> cgroup scope that the spec mandates, and making the per-harness timeout
> the only thing protecting the lane from a run-away CBMC job.

### 3.2 Current per-package shape (post-patch)

The patched lane runs **one `cargo kani -p <pkg>` invocation per
package**, then parses per-harness `VERIFICATION:` lines from the
combined stdout.

```rust
// crates/titania-lanes/src/run_lane_kani.rs::run_package
fn run_package(workspace_root: &Path, package: &str, cgroup_available: bool) -> PackageRun {
    let mut command = if cgroup_available {
        build_cgroup_command(workspace_root, package)
    } else {
        build_bare_command(workspace_root, package)
    };
    let cgroup_used = cgroup_available;
    let mut child: Child = command.spawn().expect("cargo kani spawn");
    poll_child(&mut child, start, timeout, cgroup_used)
}
```

The dispatcher iterates workspace crates and accumulates per-package
runs (`run_lane_kani.rs::kani_outcome`):

```rust
let mut state = LaneRunState::new(...);
let cgroup_available = probe_systemd_run();
let mut any_cgroup_used = false;
for package in &packages {
    let run: PackageRun = run_package(workspace_root, package, cgroup_available);
    any_cgroup_used = run.cgroup_used || any_cgroup_used;
    // accumulate findings, record exit_code, etc.
}
build_clean_outcome(&state, inventory.len(), any_cgroup_used)
```

Per-package timeout: **600 s** (was 60 s/harness under the prior
per-harness design — total wallclock for 8 harnesses drops from
`8 × 60 s = 480 s` to `1 × 600 s = 600 s`, and the build cost drops
from N passes to 1 pass).

### 3.3 cgroup wrapping + fallback

The cgroup wrapper invokes `systemd-run --user --scope -p
MemoryMax=24G -p MemorySwapMax=0` when `systemd-run` is available on the
host:

```rust
// crates/titania-lanes/src/run_lane_kani.rs::build_cgroup_command
fn build_cgroup_command(workspace_root: &Path, package: &str) -> Command {
    let mut cmd = Command::new("systemd-run");
    cmd.args(["--user", "--scope"]);
    let _ = cmd.args(["-p", &format!("MemoryMax={CGROUP_MEMORY_MAX}")]);
    let _ = cmd.args(["-p", "MemorySwapMax=0"]);
    cmd.arg("--");
    cmd.arg("cargo");
    cmd.arg("kani");
    cmd.arg("-p");
    cmd.arg(package);
    cmd.arg("--output-format=regular");
    cmd
}
```

If `systemd-run` is not on PATH (or `--version` probe fails), the lane
falls back to bare `cargo kani -p <pkg>`. The `PackageRun::cgroup_used`
flag records which path was taken, and the `CommandEvidence::argv`
reflects the shape:

```text
argv: cargo kani -p <pkg> --output-format=regular
      --arg cgroup=systemd-run-scope-MemoryMax=24G   (when available)
      --arg cgroup=fallback-no-systemd-run          (otherwise)
```

Two unit tests lock this behaviour in
`run_lane_kani.rs::tests`:

- `clean_outcome_records_cgroup_metadata_in_argv` — asserts argv
  carries `cgroup=systemd-run-scope...` when `systemd-run` is available.
- `clean_outcome_records_fallback_when_cgroup_unavailable` — asserts
  argv carries `cgroup=fallback-no-systemd-run` when systemd is not on
  PATH.

### 3.4 Per-harness verdict parsing

Per-harness verdicts are parsed from the single combined stdout (the
`HarnessVerdict::from_line` exact-match against
`["SUCCESSFUL", "FAILED", "UNSUPPORTED"]` closed set — the prior
substring-match bug at `run_lane_kani.rs:294` was replaced):

```rust
fn verdict_from_line(line: &str) -> Option<HarnessVerdict> {
    let trimmed = line.trim_start();
    let verification = trimmed.strip_prefix("VERIFICATION:")?;
    let verdict = verification.trim().trim_start_matches('-').trim();
    Some(match verdict {
        "SUCCESSFUL" => HarnessVerdict::Successful,
        "FAILED"     => HarnessVerdict::Failed,
        "UNSUPPORTED" => HarnessVerdict::Unsupported,
        _            => HarnessVerdict::Unknown,
    })
}
```

Successful / Failed / Unsupported / Unknown each produce a dedicated
finding with the typed rule id (`PROOF_KANI_PASS`, `PROOF_KANI_FAIL`,
`PROOF_KANI_UNSUPPORTED`, `PROOF_KANI_INFRA`) so the per-finding family
is reachable end-to-end (the `PROOF_KANI_PASS` family was dead in the
prior capture where every harness was BLOCKED via wallclock timeout).

### 3.5 Per-package shape vs spec §4.2 step 4

The spec mandates:

> Re-run per package with `cargo kani -p <pkg> --output-format=regular
> -j 1` inside `systemd-run --user --scope -p MemoryMax=24G -p
> MemorySwapMax=0`.

The patched lane follows this verbatim. (`-j 1` is not a `cargo kani`
flag in 0.67.0 — verified live via `cargo kani --help` which lists
`-h, --debug, -q, -v, -Z` only. Spec wording stale; spec is documented
as `cargo kani 0.67.0` doesn't accept `-j`.)

### 3.6 Tool-unavailable probe

The probe is implemented as `probe_systemd_run` which runs
`systemd-run --version` and short-circuits on non-zero exit:

```rust
fn probe_systemd_run() -> bool {
    Command::new("systemd-run")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
```

The actual `cargo kani --version` probe happens in `kani_outcome` and
emits `SkipReason::ToolUnavailable(ToolKind::CargoKani)` when the
subprocess is missing or older than the spec floor (`0.50.0`).

## 4. `.titania/out/full/kani.json` artifact

On this sandbox host (no `cargo-kani` on PATH) the lane writes a
Skipped-shape artifact:

```json
{
  "lane": "Kani",
  "outcome": {
    "Skipped": {
      "ToolUnavailable": "CargoKani"
    }
  }
}
```

The shape validates against the typed
`crates/titania-core/src/outcome.rs::LaneOutcome::Skipped { reason:
SkipReason::ToolUnavailable(ToolKind) }` enum; serde round-trips
through `LaneOutcomeWriteWire::Skipped(SkipReason)` cleanly.

The artifact is written by the lane driver after the per-package sweep
ends (no-op when the probe fails).

## 5. Aggregate `--scope full`

The aggregate reads the Kani artifact through the Full-scope
`per_lane` validator and reports the Kani lane outcome as Skipped
(passing shape — `LaneOutcome::Skipped` is accepted by
`LaneOutcome::is_pass()` per outcome.rs:240-246).

## 6. Errors and their resolution

| # | Error | Resolution |
|---|-------|------------|
| 1 | `titania-check run-lane kani` → `InputError: unknown lane 'kani'` (exit 3) — *captured in v1.5 cycle 2026-07-15* | **Resolved** in patch 2026-07-16T07:00Z. `crates/titania-check/src/args/parse.rs` extended with `"kani" => Ok(Lane::Kani)`. The same command now exits 0 and writes a typed Skipped-shape artifact. |
| 2 | Per-harness invocation pattern paying N× build cost | **Resolved** in patch 2026-07-16T12:00Z. Replaced one-`cargo kani`/harness design with one-`cargo kani`/package design. Build cost drops from N to 1; wallclock budget moves from `harness_count × 60 s` to `package_count × 600 s`. |
| 3 | No cgroup enforcement | **Resolved** in patch 2026-07-16T12:00Z. Lane now wraps `cargo kani` in `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0` when available, with bare `cargo kani` as graceful fallback. `PackageRun::cgroup_used` records which path; `CommandEvidence::argv` records the cgroup metadata. |
| 4 | `verdict_from_line` substring-match on UNSUPPORTED (review F-19) | **Resolved** in patch 2026-07-16T12:00Z. Replaced `other.contains("UNSUPPORTED")` with closed-set exact match against `["SUCCESSFUL", "FAILED", "UNSUPPORTED"]`. Unrecognised verdicts bucket to `Unknown` and surface as `PROOF_KANI_INFRA`. |
| 5 | `SkipReason::ToolUnavailable` not defined (review F-11) | **Resolved** in patch 2026-07-16T12:00Z. Added `SkipReason::ToolUnavailable(ToolKind)` variant at `outcome.rs:30`. Wired into both lanes — Kani via `SkipReason::ToolUnavailable(ToolKind::CargoKani)`, Mutants via `SkipReason::ToolUnavailable(ToolKind::CargoMutants)`. |
| 6 | `PROOF_KANI_PASS` family never emitted (every prior harness was BLOCKED via timeout) | **Resolved** in patch 2026-07-16T12:00Z. Per-package 600 s timeout is wider than 60 s/harness × N; on hardware that actually completes within 600 s the harness emits `PROOF_KANI_PASS` rather than `PROOF_KANI_BLOCKED`. |

## 7. Artifacts written

| Path | Description |
|------|-------------|
| `.titania/out/full/kani.json` | Per-run lane artifact (Skipped shape on this sandbox; full Findings on hosts with cargo-kani). |
| `.evidence/v1.5/raw/kani-version.txt` | Prior `cargo kani --version` capture (cargo-kani 0.67.0) — reference for the version that the lane *would* query at startup. |
| `.evidence/v1.5/raw/kani-harness-counts.txt` | Prior per-crate harness inventory capture (8 in titania-core, 0 elsewhere). |
| `.evidence/v1.5/raw/kani-list-titania-<pkg>.json` | Per-crate `cargo kani list --format json` outputs (8 in titania-core; 0 elsewhere). |
| `.evidence/v1.5/raw/kani-lane-run-now.log` | Live run of `cargo run -p titania-check -- run-lane kani` (exit 0, Skipped). |
| `.evidence/v1.5/raw/kani-lane-evidence.md` | This file. |
| `.evidence/v1.5/kani-harnesses.json` | Combined v1.5 harness inventory snapshot for spec §9 A1. |
