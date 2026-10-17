# Regression diff — tn-f7ti

writer_skill: formal-verifier
writer_invocation_id: INV-14-formal-verifier

Base: 927893f922fe2b9c658a0c1574086afe7d6a6dc6. Working-tree delta:

- M contracts/proof_obligations.yaml (authority marking only).
- M crates/titania-lanes/src/lib.rs (one module line).
- New crates/titania-lanes/src/proof_registry.rs.
- New crates/titania-lanes/tests/proof_registry.rs.

No other production file changed. Full-workspace check and strict source
clippy pass, so no new warnings were introduced anywhere. The touched
crate suite delta is +13 passing checks; the one failing check
(dylint_reject_repair) fails identically with the delta stashed, hence
BLOCK_GLOBAL pre-existing, not a regression.
