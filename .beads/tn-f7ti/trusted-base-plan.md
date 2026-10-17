# Trusted-base plan — tn-f7ti

The subject proof file verification/verus/receipt_schema.rs bridges to
production through the Verus assume_specification form over
production::is_supported_receipt_schema_version. That marker is
pre-existing subject content, not a claim authored by this bead.

Plan: record one trusted-base-ledger row (TBL-assume-spec) naming the
artifact, location, marker text, and kind, with behavior_affecting false
(the bead changes no production semantics) and compensating evidence (the
audit classifies the file by its own token scan, and the behavior suite
pins both content classes). No other trust markers are introduced by this
bead: the implementation contains no assume/admit/axiom-style content and
no disabled checks.

Ledger: trusted-base-ledger.jsonl, reviewer disposition accepted.
