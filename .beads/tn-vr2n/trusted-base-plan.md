# Trusted base plan — tn-vr2n

No `assume`/`trusted`/`external_body` markers are planned in any proof or
test artifact. Behavior evidence rests on: the Rust compiler, cargo test
harness, and the checked-in deterministic fixtures. If any trust marker
appears during writing, it must be ledgered in trusted-base-ledger.jsonl
before state 12; none are expected.
