#!/usr/bin/env python3
"""
Check v1 release evidence manifest completeness.

Usage: python .evidence/v1-release/check_evidence.py .evidence/v1-release/manifest.toml

Exits 0 if all DoD IDs 1-14 appear exactly once and every entry has the
required keys. Exits 1 otherwise.
"""

import sys


REQUIRED_KEYS = {
    "id", "spec_ref", "evidence_path", "command",
    "exit_status", "review_status", "blocker_reason",
}
EXPECTED_IDS = set(range(1, 15))


def parse_manifest(path: str) -> list[dict[str, str]]:
    """Parse a simple TOML manifest with [[dod]] entries."""
    entries: list[dict[str, str]] = []
    current: dict[str, str] | None = None
    with open(path, "r") as f:
        for line in f:
            stripped = line.strip()
            if stripped == "[[dod]]":
                if current is not None:
                    entries.append(current)
                current = {}
            elif stripped.startswith("#") or stripped == "":
                continue
            elif stripped.startswith("[") and not stripped.startswith("[["):
                # Table header (e.g. [workspace]) — discard current entry
                current = None
            elif "=" in stripped and current is not None:
                key, _, value = stripped.partition("=")
                key = key.strip()
                value = value.strip().strip('"')
                if key in REQUIRED_KEYS:
                    current[key] = value
    if current is not None:
        entries.append(current)
    return entries


def main() -> int:
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <manifest.toml>", file=sys.stderr)
        return 1

    manifest_path = sys.argv[1]

    try:
        entries = parse_manifest(manifest_path)
    except Exception as e:
        print(f"FAIL: Could not parse manifest: {e}", file=sys.stderr)
        return 1

    # Check DoD IDs
    found_ids: set[int] = set()
    for entry in entries:
        try:
            id_val = int(entry.get("id", "0"))
        except (ValueError, TypeError):
            print(f"FAIL: Entry has non-integer id: {entry}", file=sys.stderr)
            return 1
        if id_val in found_ids:
            print(f"FAIL: Duplicate DoD ID {id_val}", file=sys.stderr)
            return 1
        found_ids.add(id_val)

    missing_ids = EXPECTED_IDS - found_ids
    if missing_ids:
        print(f"FAIL: Missing DoD IDs: {sorted(missing_ids)}", file=sys.stderr)
        return 1
    # Validate .exit files against manifest exit_status
    import os
    manifest_dir = os.path.dirname(manifest_path)
    for entry in entries:
        try:
            expected_exit = int(entry.get("exit_status", "0"))
        except (ValueError, TypeError):
            print(f"FAIL: Entry id={entry.get('id')} has non-integer exit_status", file=sys.stderr)
            return 1
        # Look for matching .exit file
        dod_id = entry.get("id", "?")
        exit_file = os.path.join(manifest_dir, "raw", f"dod{int(dod_id):02d}.exit")
        if os.path.isfile(exit_file):
            try:
                actual_exit = int(open(exit_file).read().strip())
                if actual_exit != expected_exit:
                    print(f"FAIL: DoD {dod_id} exit_status mismatch: manifest={expected_exit} file={actual_exit}", file=sys.stderr)
                    return 1
            except ValueError:
                print(f"FAIL: DoD {dod_id} .exit file contains non-integer: {open(exit_file).read().strip()}", file=sys.stderr)
                return 1
        # else: no .exit file — skip (optional)

    # Check required keys in each entry
    for entry in entries:
        entry_id = entry.get("id", "?")
        missing_keys = REQUIRED_KEYS - set(entry.keys())
        if missing_keys:
            print(f"FAIL: DoD {entry_id} missing keys: {sorted(missing_keys)}", file=sys.stderr)
            return 1

    print(
        f"PASS: All {len(entries)} DoD entries "
        f"(IDs {sorted(found_ids)}) present with all required keys."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
