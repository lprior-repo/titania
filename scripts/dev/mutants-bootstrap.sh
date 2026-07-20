#!/usr/bin/env bash
# =============================================================================
# mutants-bootstrap.sh — v1.5 cargo-mutants zero-survivor baseline bootstrap.
#
# Implements `.evidence/v1.5/spec.md §4.4 Baseline bootstrap (D3)`:
#
#   For every target package, run `cargo mutants --no-shuffle --output mutants.out
#   -p <pkg>` in FULL TEST MODE (no `--check` — see spec §4.3 / R3). Parse
#   `mutants.out/outcomes.json` for the cargo-mutants `SummaryOutcome::MissedMutant`
#   scenarios, and append every such surviving mutation-id to
#   `.titania/profiles/strict-ai/mutants.baseline.json` as a typed
#   `MutantBaselineEntry` (`mutation_id`, `accepted_by_rule`,
#   `reason`, `expires_on_unix`). The file is `schema_version: 1` to match
#   `titania-core/src/mutants_baseline.rs::MUTANTS_BASELINE_SCHEMA_VERSION`.
#
# Properties:
#   * Idempotent: re-runs do not duplicate entries (mutation_ids already
#     present in the baseline are skipped).
#   * Non-destructive: existing entries are preserved verbatim.
#   * Atomic: writes go to `<baseline>.tmp` then `os.replace` into place.
#   * Idempotent on missing file: a fresh checkout with no baseline gets a
#     new baseline with `schema_version: 1`.
#   * JSON-safe: writes via `python3`'s `json` module — no heredoc string
#     concatenation that could corrupt the document.
#
# Usage: mutants-bootstrap.sh --owner <name> --reason <text> [OPTIONS]
#        mutants-bootstrap.sh --help
#
# See `usage()` below for the full option list.
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Constants (mirror titania-core/src/mutants_baseline.rs).
# -----------------------------------------------------------------------------
readonly DEFAULT_BASELINE_PATH=".titania/profiles/strict-ai/mutants.baseline.json"
readonly EXPECTED_SCHEMA_VERSION=1
readonly MUTANTS_OUTPUT_DIR="mutants.out"
readonly SCRIPT_NAME="$(basename "$0")"

# -----------------------------------------------------------------------------
# Pretty output (kept simple — no tput dependency).
# -----------------------------------------------------------------------------
log_info()  { printf '[%s] INFO  %s\n'  "$SCRIPT_NAME" "$*" >&2; }
log_warn()  { printf '[%s] WARN  %s\n'  "$SCRIPT_NAME" "$*" >&2; }
log_error() { printf '[%s] ERROR %s\n'  "$SCRIPT_NAME" "$*" >&2; }
die()       { log_error "$*"; exit 1; }

# -----------------------------------------------------------------------------
# Usage.
# -----------------------------------------------------------------------------
usage() {
    cat <<USAGE
Usage: ${SCRIPT_NAME} --owner <name> --reason <text> [OPTIONS]

Bootstrap the v1.5 cargo-mutants baseline by running cargo mutants in full
test-mode against the requested package(s) and appending every survivor to
the baseline JSON as an accepted-by-rule entry.

Options:
  --package <pkg>       Target a specific workspace package. Repeat the flag
                        to target multiple packages. Default: all workspace
                        members (resolved via \`cargo metadata --no-deps\`).
  --baseline <path>     Baseline JSON path.
                        Default: ${DEFAULT_BASELINE_PATH}
  --owner <name>        Owner recorded in every \`accepted_by_rule\` field.
                        Required. Example: titania-maintainers.
  --reason <text>       Human-readable reason recorded in every entry.
                        Required. Will be embedded into the rule id as
                        \`mutant-accept/<owner>/<reason>/never\`.
  --dry-run             Run cargo mutants and report what would be appended,
                        but do NOT modify the baseline file.
  -h, --help            Print this help and exit.

Exit codes:
  0   success (baseline updated or no-op)
  1   fatal precondition failure (tool missing, bad args, bad baseline)
  2   baseline on disk has incompatible schema_version
  3   cargo-mutants run failed or produced no parseable outcomes

Examples:
  # Bootstrap every workspace crate with owner + reason.
  ${SCRIPT_NAME} --owner titania-maintainers --reason "v1.5 initial sweep"

  # Bootstrap a single package, preview-only.
  ${SCRIPT_NAME} --package titania-core --owner lewis --reason "audit" --dry-run

  # Use a custom baseline path.
  ${SCRIPT_NAME} --owner titania-maintainers --reason "v1.5 initial sweep" \\
                  --baseline .titania/profiles/strict-ai/mutants.baseline.json
USAGE
}

# -----------------------------------------------------------------------------
# Argument parsing.
# -----------------------------------------------------------------------------
PACKAGES=()
BASELINE_PATH="${DEFAULT_BASELINE_PATH}"
OWNER=""
REASON=""
DRY_RUN="false"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --package)
            [[ $# -ge 2 ]] || die "--package requires a value"
            PACKAGES+=("$2")
            shift 2
            ;;
        --package=*)
            PACKAGES+=("${1#*=}")
            shift
            ;;
        --baseline)
            [[ $# -ge 2 ]] || die "--baseline requires a value"
            BASELINE_PATH="$2"
            shift 2
            ;;
        --baseline=*)
            BASELINE_PATH="${1#*=}"
            shift
            ;;
        --owner)
            [[ $# -ge 2 ]] || die "--owner requires a value"
            OWNER="$2"
            shift 2
            ;;
        --owner=*)
            OWNER="${1#*=}"
            shift
            ;;
        --reason)
            [[ $# -ge 2 ]] || die "--reason requires a value"
            REASON="$2"
            shift 2
            ;;
        --reason=*)
            REASON="${1#*=}"
            shift
            ;;
        --dry-run)
            DRY_RUN="true"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            break
            ;;
        -*)
            die "unknown option: $1 (use --help)"
            ;;
        *)
            die "unexpected positional argument: $1 (use --help)"
            ;;
    esac
done

# Required-arg gate (after parsing so --help works without them).
if [[ -z "${OWNER}" ]]; then
    die "--owner is required (use --help)"
fi
if [[ -z "${REASON}" ]]; then
    die "--reason is required (use --help)"
fi

# -----------------------------------------------------------------------------
# Preconditions.
# -----------------------------------------------------------------------------
command -v cargo >/dev/null 2>&1 \
    || die "cargo not found in PATH"
command -v python3 >/dev/null 2>&1 \
    || die "python3 not found in PATH (required for safe JSON I/O)"
command -v cargo-mutants >/dev/null 2>&1 \
    || die "cargo-mutants not found in PATH; install with: cargo install cargo-mutants --locked"

# -----------------------------------------------------------------------------
# Resolve target packages.
# -----------------------------------------------------------------------------
resolve_workspace_packages() {
    cargo metadata --no-deps --format-version=1 \
        | python3 -c '
import json, sys
data = json.load(sys.stdin)
for pkg in data.get("packages", []):
    name = pkg.get("name")
    if name:
        print(name)
'
}

if [[ ${#PACKAGES[@]} -eq 0 ]]; then
    log_info "no --package given; defaulting to all workspace members"
    while IFS= read -r pkg; do
        [[ -n "${pkg}" ]] && PACKAGES+=("${pkg}")
    done < <(resolve_workspace_packages)
fi

if [[ ${#PACKAGES[@]} -eq 0 ]]; then
    die "no packages resolved; pass --package or run from a Cargo workspace root"
fi

log_info "target packages (${#PACKAGES[@]}): ${PACKAGES[*]}"
log_info "baseline path:           ${BASELINE_PATH}"
log_info "owner:                   ${OWNER}"
log_info "reason:                  ${REASON}"
if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "mode:                    DRY-RUN (baseline will NOT be modified)"
fi

# -----------------------------------------------------------------------------
# Extract surviving mutation-ids from a cargo-mutants outcomes.json.
#
# Per spec §4.4 we run full test-mode (no --check). Survivors are scenarios
# with summary == "MissedMutant" in `mutants.out/outcomes.json`. The
# cargo-mutants `Mutant.name` field is the stable mutation-id we record.
# -----------------------------------------------------------------------------
extract_survivors() {
    local outcomes_json="$1"
    python3 - "${outcomes_json}" <<'PYEOF'
import json
import sys

path = sys.argv[1]
try:
    with open(path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except FileNotFoundError:
    print(f"ERROR: {path} not found", file=sys.stderr)
    sys.exit(3)
except json.JSONDecodeError as exc:
    print(f"ERROR: {path} is not valid JSON: {exc}", file=sys.stderr)
    sys.exit(3)

for entry in data.get("outcomes", []):
    if entry.get("summary") != "MissedMutant":
        continue
    scenario = entry.get("scenario")
    if not isinstance(scenario, dict):
        continue
    mutant = scenario.get("Mutant")
    if not isinstance(mutant, dict):
        continue
    name = mutant.get("name")
    if name:
        print(name)
PYEOF
}

# Count generated mutants per package (entries with a Mutant scenario).
count_generated_mutants() {
    local outcomes_json="$1"
    python3 - "${outcomes_json}" <<'PYEOF'
import json
import sys

path = sys.argv[1]
try:
    with open(path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except (FileNotFoundError, json.JSONDecodeError):
    print(0)
    sys.exit(0)

total = 0
for entry in data.get("outcomes", []):
    scenario = entry.get("scenario")
    if isinstance(scenario, dict) and "Mutant" in scenario:
        total += 1
print(total)
PYEOF
}

# -----------------------------------------------------------------------------
# Run cargo mutants per package, collect survivors.
# -----------------------------------------------------------------------------
ALL_SURVIVORS=()
declare -A PER_PKG_GENERATED=()
declare -A PER_PKG_SURVIVORS=()

for pkg in "${PACKAGES[@]}"; do
    log_info "==> running cargo mutants (full test-mode) for package: ${pkg}"
    log_info "    command: cargo mutants --no-shuffle --output ${MUTANTS_OUTPUT_DIR} -p ${pkg}"

    pkg_log="$(mktemp -t "mutants-log.XXXXXX")"

    # set +e to capture cargo-mutants' exit code; re-enable afterwards.
    set +e
    cargo mutants --no-shuffle --output "${MUTANTS_OUTPUT_DIR}" -p "${pkg}" \
        >"${pkg_log}" 2>&1
    rc=$?
    set -e

    if [[ ${rc} -ne 0 ]]; then
        log_warn "cargo mutants exited non-zero (rc=${rc}) for ${pkg}; log: ${pkg_log}"
    fi

    outcomes_path="${MUTANTS_OUTPUT_DIR}/outcomes.json"
    if [[ ! -f "${outcomes_path}" ]]; then
        log_warn "no outcomes.json at ${outcomes_path} for ${pkg}; skipping"
        PER_PKG_GENERATED["${pkg}"]=0
        PER_PKG_SURVIVORS["${pkg}"]=0
        rm -f "${pkg_log}"
        continue
    fi

    pkg_generated="$(count_generated_mutants "${outcomes_path}" | tr -d '[:space:]')"
    PER_PKG_GENERATED["${pkg}"]="${pkg_generated:-0}"

    pkg_survivor_count=0
    while IFS= read -r mid; do
        [[ -n "${mid}" ]] || continue
        ALL_SURVIVORS+=("${mid}")
        pkg_survivor_count=$(( pkg_survivor_count + 1 ))
    done < <(extract_survivors "${outcomes_path}")
    PER_PKG_SURVIVORS["${pkg}"]="${pkg_survivor_count}"

    rm -f "${pkg_log}"
done

# -----------------------------------------------------------------------------
# Per-package + aggregate counters.
# -----------------------------------------------------------------------------
total_generated=0
total_survivors="${#ALL_SURVIVORS[@]}"

log_info "==> summary of cargo-mutants runs"
for pkg in "${PACKAGES[@]}"; do
    log_info "    ${pkg}: generated=${PER_PKG_GENERATED[${pkg}]:-0}  survivors=${PER_PKG_SURVIVORS[${pkg}]:-0}"
    total_generated=$(( total_generated + ${PER_PKG_GENERATED[${pkg}]:-0} ))
done

# -----------------------------------------------------------------------------
# --dry-run: do everything except touch the baseline file.
# -----------------------------------------------------------------------------
if [[ "${DRY_RUN}" == "true" ]]; then
    log_info "DRY-RUN: not modifying baseline file at ${BASELINE_PATH}"

    # Count current entries (after the run would have been applied — same as
    # before, since we're not modifying).
    current_entries="$(python3 - "${BASELINE_PATH}" <<'PYEOF'
import json
import os
import sys

path = sys.argv[1]
if not os.path.exists(path):
    print(0)
    sys.exit(0)
try:
    with open(path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except json.JSONDecodeError:
    print(0)
    sys.exit(0)
print(len(data.get("entries", [])))
PYEOF
)"

    if [[ ${total_survivors} -eq 0 ]]; then
        log_info "DRY-RUN: nothing to add"
        printf 'discovered=%d survivors=%d appended=0 baseline_entries=%s\n' \
            "${total_generated}" "${total_survivors}" "${current_entries}"
        exit 0
    fi
    log_info "DRY-RUN: would append the following ${total_survivors} mutation_ids:"
    printf '  %s\n' "${ALL_SURVIVORS[@]}" >&2
    printf 'discovered=%d survivors=%d appended=0 baseline_entries=%s\n' \
        "${total_generated}" "${total_survivors}" "${current_entries}"
    exit 0
fi

# -----------------------------------------------------------------------------
# Update the baseline JSON atomically.
#
# The python script:
#   * reads (or initialises) the existing baseline,
#   * validates schema_version == 1,
#   * dedupes new mutation_ids against existing entries + within-run,
#   * writes the merged document to <baseline>.tmp,
#   * uses os.replace() for the atomic move into place,
#   * prints two structured counters on stdout for the bash wrapper to read:
#         appended=<n>
#         baseline_entries_after=<n>
# -----------------------------------------------------------------------------
baseline_tmp="${BASELINE_PATH}.tmp"

# Cleanup any stale tmp from a previous interrupted run.
rm -f "${baseline_tmp}"

# Clean up our tmp if we exit before the python rename succeeds.
trap 'rm -f "${baseline_tmp}"' EXIT

PY_OUTPUT="$(
    python3 - "${BASELINE_PATH}" "${baseline_tmp}" "${OWNER}" "${REASON}" "${ALL_SURVIVORS[@]}" \
        <<'PYEOF'
import datetime
import json
import os
import sys

baseline_path   = sys.argv[1]
tmp_path        = sys.argv[2]
owner           = sys.argv[3]
reason          = sys.argv[4]
new_mutation_ids = sys.argv[5:]

EXPECTED_SCHEMA_VERSION = 1


def load_baseline(path):
    if not os.path.exists(path):
        return {"schema_version": EXPECTED_SCHEMA_VERSION, "entries": []}
    try:
        with open(path, "r", encoding="utf-8") as fh:
            data = json.load(fh)
    except json.JSONDecodeError as exc:
        print(f"ERROR: {path} is not valid JSON: {exc}", file=sys.stderr)
        sys.exit(1)
    if not isinstance(data, dict):
        print(f"ERROR: {path} is not a JSON object", file=sys.stderr)
        sys.exit(1)
    if "schema_version" not in data:
        print(f"ERROR: {path} missing required field 'schema_version'", file=sys.stderr)
        sys.exit(1)
    if data["schema_version"] != EXPECTED_SCHEMA_VERSION:
        print(
            f"ERROR: {path} has schema_version={data['schema_version']!r}; "
            f"expected {EXPECTED_SCHEMA_VERSION}",
            file=sys.stderr,
        )
        sys.exit(2)
    if "entries" not in data or not isinstance(data["entries"], list):
        print(
            f"ERROR: {path} is missing a list-valued 'entries' field",
            file=sys.stderr,
        )
        sys.exit(1)
    return data


def save_baseline_atomic(data, tmp_path, final_path):
    with open(tmp_path, "w", encoding="utf-8") as fh:
        json.dump(data, fh, indent=2, ensure_ascii=False, sort_keys=False)
        fh.write("\n")
    os.replace(tmp_path, final_path)


data = load_baseline(baseline_path)

existing_ids = {
    entry.get("mutation_id")
    for entry in data["entries"]
    if isinstance(entry, dict) and entry.get("mutation_id")
}

accepted_by_rule = f"mutant-accept/{owner}/{reason}/never"

appended = 0
seen_in_run = set()
for mid in new_mutation_ids:
    if not mid:
        continue
    if mid in existing_ids or mid in seen_in_run:
        continue
    data["entries"].append({
        "mutation_id":       mid,
        "accepted_by_rule":  accepted_by_rule,
        "reason":            reason,
        "expires_on_unix":   None,
    })
    existing_ids.add(mid)
    seen_in_run.add(mid)
    appended += 1

if appended > 0:
    data["computed_at"] = (
        datetime.datetime.now(datetime.timezone.utc)
        .strftime("%Y-%m-%dT%H:%M:%SZ")
    )
    save_baseline_atomic(data, tmp_path, baseline_path)
else:
    # No-op: do not rewrite the baseline (no file modification, no mtime bump).
    # Still remove a stale tmp just in case.
    if os.path.exists(tmp_path):
        os.remove(tmp_path)

# Structured counters on stdout — bash reads these two lines to print the
# final summary.
print(f"appended={appended}")
print(f"baseline_entries_after={len(data['entries'])}")
PYEOF
)"

# -----------------------------------------------------------------------------
# Final summary on stdout (single line, easy to grep / parse from CI).
# Parse the counters the python script printed.
# -----------------------------------------------------------------------------
APPENDED="$(printf '%s\n' "${PY_OUTPUT}" | awk -F= '/^appended=/{print $2; exit}')"
BASELINE_TOTAL="$(printf '%s\n' "${PY_OUTPUT}" | awk -F= '/^baseline_entries_after=/{print $2; exit}')"

# If parsing failed (empty), fall back to 0 / 0.
: "${APPENDED:=0}"
: "${BASELINE_TOTAL:=0}"

printf 'discovered=%d survivors=%d appended=%d baseline_entries=%d\n' \
    "${total_generated}" "${total_survivors}" "${APPENDED}" "${BASELINE_TOTAL}"