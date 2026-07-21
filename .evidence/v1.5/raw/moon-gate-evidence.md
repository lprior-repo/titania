# v1.5 Full Gate — Moon Run Evidence

**Date:** 2026-07-15
**Workspace:** `/home/lewis/src/titania`
**Operator:** opencode (manual Moon v2 orchestration)
**Scope:** Attempt to run the v1.5 `gate-full` composite and its dependent lanes
(`titania-kani`, `titania-mutants`) via Moon v2.

---

## 1. Moon Version

```
$ which moon && moon --version
/home/lewis/.local/share/mise/installs/npm-moonrepo-cli/2.2.4/bin/moon
moon 2.2.4
```

Moon **is installed** at `2.2.4`. Raw artifact: `moon-version.txt`.

The original prompt invoked the task as `moon :titania:gate-full`. That
syntax is **not valid** for this Moon version. The correct invocation is
`moon run titania:<task-id>`. All captures below use the corrected form;
the raw failed invocation is preserved in `moon-gate-full.log` for the
record.

---

## 2. Gate Definition (`.moon/tasks/all.yml`)

The composite task exists at line 376 of `.moon/tasks/all.yml`:

```yaml
gate-full:
  # v1.5 Full gate — release + Kani + Mutants. PROOF_KANI_* and
  # MUTANT_SURVIVED findings surface in the aggregate report.
  command: 'cargo run --frozen --quiet -p titania-check -- aggregate --scope full'
  deps: ['~:gate-release', '~:titania-kani', '~:titania-mutants']
  options:
    runInCI: true
```

Raw artifact: `gate-full-yaml.txt`, `gate-full-tasks-all.yml.txt`.

It depends on three sub-tasks:
- `gate-release` (prepush + build)
- `titania-kani` (`.moon/tasks/all.yml:320`) — invokes
  `cargo run --frozen --quiet -p titania-check -- run-lane kani`
- `titania-mutants` (`.moon/tasks/all.yml:339`) — invokes
  `cargo run --frozen --quiet -p titania-check -- run-lane mutants`

---

## 3. Per-Task Status

| Task | Command | Exit Code | Status |
|------|---------|-----------|--------|
| `moon run titania:gate-full`  | (composite) | **1** | **FAIL** — kani lane failed early; aggregate never ran |
| `moon run titania:titania-kani`    | direct | **3** (`InputError`) | **FAIL** — `unknown lane 'kani'` |
| `moon run titania:titania-mutants` | direct | **3** (`InputError`) | **FAIL** — `unknown lane 'mutants'` |

### 3.1 `gate-full` (composite) — exit 1

```
▮▮▮▮ titania:titania-kani (aee1c962)
InputError: unknown lane 'kani'
▮▮▮▮ titania:titania-kani (157ms, aee1c962)
…
Error: task_runner::run_failed
  × Task titania:titania-kani failed to run.
  ╰─▶ Process cargo failed: exit code 3
```

The composite is wired correctly: it depended on `titania-kani`, which
exited non-zero, so Moon aborted the pipeline. The downstream
`aggregate --scope full` step never executed. Raw artifact:
`moon-gate-full.log`.

### 3.2 `titania-kani` — exit 3 (`InputError`)

```
▮▮▮▮ titania:titania-kani (aee1c962)
InputError: unknown lane 'kani'
▮▮▮▮ titania:titania-kani (55ms, aee1c962)
Error: task_runner::run_failed
  × Task titania:titania-kani failed to run.
  ╰─▶ Process cargo failed: exit code 3
```

Raw artifact: `moon-titania-kani.log`.

### 3.3 `titania-mutants` — exit 3 (`InputError`)

```
▮▮▮▮ titania:titania-mutants (e335cbc8)
InputError: unknown lane 'mutants'
▮▮▮▮ titania:titania-mutants (53ms, e335cbc8)
Error: task_runner::run_failed
  × Task titania:titania-mutants failed to run.
  ╰─▶ Process cargo failed: exit code 3
```

Raw artifact: `moon-titania-mutants.log`.

> **Note:** This is **not** the expected `MUTANT_SURVIVED` failure
> described in the original prompt. The mutants lane never reached the
> `cargo mutants` execution stage. See §4 for the root cause.

---

## 4. Root Cause — `titania-check run-lane` rejects `kani` / `mutants`

The Moon task definitions ask `titania-check` to dispatch a `kani` or
`mutants` lane. The CLI rejects both because its argument parser does not
register those lane names.

### 4.1 CLI self-report

```
$ cargo run --frozen --quiet -p titania-check -- run-lane --help
…
LANES (run-lane):
    fmt, compile, clippy, ast-grep, dylint, panic-scan, policy-scan,
    test, deny, build
```

`kani` and `mutants` are not in the list.

### 4.2 Source-level evidence

`crates/titania-check/src/args/parse.rs:458-472` — `parse_lane()` is
missing match arms for `"kani"` and `"mutants"`:

```rust
fn parse_lane(value: &str) -> Result<Lane, CliError> {
    match value {
        "fmt" => Ok(Lane::Fmt),
        "compile" => Ok(Lane::Compile),
        "clippy" => Ok(Lane::Clippy),
        "ast-grep" => Ok(Lane::AstGrep),
        "dylint" => Ok(Lane::Dylint),
        "panic-scan" => Ok(Lane::PanicScan),
        "policy-scan" => Ok(Lane::PolicyScan),
        "test" => Ok(Lane::Test),
        "deny" => Ok(Lane::Deny),
        "build" => Ok(Lane::Build),
        _ => Err(CliError::UnknownLane(value.to_owned())),
    }
}
```

Meanwhile, `crates/titania-check/src/main.rs:285-286` already wires the
`Lane` enum:

```rust
Lane::Kani => "kani",
Lane::Mutants => "mutants",
```

…so the enum and the `lane_stem()` mapping already know about these
lanes; the CLI argument parser and `execute_lane()` dispatcher are the
gap. The kani/mutants lanes are referenced from
`crates/titania-check/src/moon.rs:160` (`FULL_TASKS`) and from the Moon
task YAML but cannot be reached via the CLI.

The same fix may also be required in any `execute_lane()` arm that
dispatches on `Lane::Kani` / `Lane::Mutants`, but this report stops at
argument parsing per the no-source-modification rule.

---

## 5. Deviation From The Prompted Expectation

The original request assumed the only non-zero exit would be from
`MUTANT_SURVIVED` findings (i.e. the mutants lane reached the
`cargo mutants` stage and reported survivors because
`mutants.baseline.json` is empty). That is **not** what happened.

Both `titania-kani` and `titania-mutants` exit early in the CLI argument
parser with `InputError: unknown lane '<name>'` (exit code **3**),
**before** any cargo / kani / mutants work runs. As a result:

- The kani lane cannot exercise any harness.
- The mutants lane cannot exercise the baseline-vs-current comparison.
- The `gate-full` aggregate never executes.

`scripts/dev/mutants-bootstrap.sh` is therefore not the remediation for
this run — the baseline bootstrap is downstream of the broken parser.
Fixing the parser to accept `"kani"` and `"mutants"` and dispatching them
in `execute_lane()` is the prerequisite.

---

## 6. Remediation Plan

Per the no-source-modification rule, this report only documents the fix
shape — it does **not** apply it.

1. **Parser fix (required).** Extend `parse_lane()` in
   `crates/titania-check/src/args/parse.rs:458` with two new match arms:

   ```rust
   "kani" => Ok(Lane::Kani),
   "mutants" => Ok(Lane::Mutants),
   ```

2. **Help text fix (required).** Update the `LANES (run-lane)` block in
   `crates/titania-check/src/args.rs:306` so the `titania-check
   run-lane --help` output matches the new arms (add `kani, mutants`).

3. **Dispatcher audit (required).** Verify that
   `titania_lanes::run_lane::execute_lane()` (called from
   `crates/titania-check/src/main.rs:401`) handles `Lane::Kani` and
   `Lane::Mutants`. If it currently does not, add the arms. Confirm
   that the artifact paths used by `aggregate --scope full` (i.e.
   `.titania/out/full/kani.json`, `.titania/out/full/mutants.json`) match
   what those new arms write.

4. **Re-run the gate.** After steps 1–3 land, re-run `moon run
   titania:gate-full`. At that point the **expected** behavior
   documented in the original prompt becomes operative:
   - `titania-kani` should produce `PROOF_KANI_*` findings (none today
     because no harness exists yet for the Full scope — verify by
     inspecting `.titania/out/full/kani.json`).
   - `titania-mutants` should produce `MUTANT_SURVIVED` findings
     because `mutants.baseline.json` is empty; populate it via
     `scripts/dev/mutants-bootstrap.sh` to clear those findings.

5. **Regression gate (required).** After the parser fix, run the
   Holzmann-Rust source-only gate to confirm no clippy / rustdoc
   regressions are introduced:

   ```bash
   cargo fmt --all -- --check
   cargo check --workspace --all-targets --all-features
   cargo clippy --workspace --lib --bins --examples --all-features \
     -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery \
        -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
   cargo nextest run --workspace --all-features
   ```

---

## 7. Artifact Index

| File | Description |
|------|-------------|
| `moon-version.txt`                     | `which moon && moon --version` output |
| `gate-full-yaml.txt`                   | `grep -A6 "gate-full" .moon/tasks/all.yml` raw output |
| `gate-full-tasks-all.yml.txt`          | Copy of the same snippet for archival |
| `moon-gate-full.log`                   | `moon run titania:gate-full --log error` (exit 1) |
| `moon-titania-kani.log`                | `moon run titania:titania-kani --log error` (exit 3) |
| `moon-titania-mutants.log`             | `moon run titania:titania-mutants --log error` (exit 3) |
| `moon-gate-evidence.md`                | This document |

All artifacts live under `/home/lewis/src/titania/.evidence/v1.5/raw/`.