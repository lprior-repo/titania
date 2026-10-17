# Quota-incident note — tn-vr2n (supplementary, post-gate)

During final re-verification, `cargo test -p titania-lanes --lib` failed
7 tests with OS error 122 (QuotaExceeded) from `tempfile::tempdir()`.
Cause: transient `/tmp` quota pressure from other machine activity
(3923 entries, 50G used; shell probes of `/tmp` intermittently failed
while `/var/tmp` stayed healthy). Not a code regression: pre-existing
tests failed identically, and the same tree passes fully with
`TMPDIR=/var/tmp/tnvr2n-tmp`.

Corroborating re-run on the final (byte-identical, post-stash-pop) tree:
`TMPDIR=/var/tmp/tnvr2n-tmp cargo test -p titania-lanes --all-features`
→ exit 0, 29/29 targets ok, 0 failed. Log:
`.beads/tn-vr2n/logs/gate-test-lanes-final.log`.

The state-12 ledger still references `gate-test-lanes.log`, which was
produced from byte-identical sources before the stash-verify cycles; both
logs are retained. Scratch dir `/var/tmp/tnvr2n-tmp` removed after use.
