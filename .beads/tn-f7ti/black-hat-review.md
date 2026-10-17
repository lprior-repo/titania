# Black-hat review — tn-f7ti

reviewer_skill: black-hat-reviewer
reviewer_invocation_id: INV-15-black-hat-reviewer
writer_invocation_id: INV-13-holzman-rust

STATUS: APPROVED

## Proof/Test/Source Parity Matrix

| Claim | Proof evidence | Test evidence | Source evidence | Verdict |
|-------|----------------|---------------|-----------------|---------|
| Registered missing/unreadable/proof-free fails named | PO-RC-* PASS rows | deleted-file, proof-free checks with exact variants | check_registered_entry, check_entry_content | holds |
| Removed required row blocks | PO-RC-flux PASS row | removed-row check | check_required_rows + REQUIRED_PRODUCTION_OBLIGATIONS | holds |
| Smoke never counts as production | PO-SD-verus PASS row | smoke-rejection, smoke-tag checks | authority fit in check_entry_content | holds |
| Unregistered proof needs disposition | PO-SD-flux PASS row | unregistered-disposition check + live inventory | discovery sweep in audit/audit_workspace | holds |
| Registry integrity | PO-RC-proptest PASS row | unknown-field, checked-in-registry checks | parse_registry strict schema | holds |
| No comment/string/extension theater | PO-SD-kani PASS row | comment-string, extension, vr-content checks | scanner + classify_source | holds |

Attacks attempted: comment-planted tokens (rejected by stripping);
string-planted tokens (rejected); smoke registered as production
(rejected by name); required row deleted (blocked); unknown registry key
(rejected at read); duplicate path (rejected); empty authority attach
(rejected). No parity gap found: every contract clause has proof, test,
and source rows that agree. The executable-check substitution is open and
reviewed, not hidden. The pre-existing dylint failure is outside this
bead and fails on base too.
