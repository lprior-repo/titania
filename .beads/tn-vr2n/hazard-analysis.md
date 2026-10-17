# Hazard analysis — tn-vr2n

H1. Ceiling smuggled via `extra_args` → silent Clean receipt with neutered
denies. Control: chokepoint guard (new); verified by failing-first behavior
suite + wiring unit test that never spawns.

H2. Static argv gains a ceiling token in a later edit. Control: negative
unit test over every lane argv + existing exact-match pin test.

H3. Env/config ceiling (`RUSTFLAGS`, encoded flags, `.cargo/config`
rustflags, wrappers). Control: existing scanner rejections + spawn scrub;
locked by new `--cap-lints`-specific regressions. Residual: `CARGO_BUILD_*`
env forms are unscanned but scrubbed at spawn — analyzed, not a live bypass.

H4. Malicious target supplies its own weaker validator receipt. Control:
receipts only originate from the protected runner (`CommandEvidence`
built inside `clean_outcome`); guard runs before any receipt exists.

H5. Over-broad matcher blocks legitimate flags (`--cap-lints` as substring
of an unrelated flag). Control: exact-token + `=`-form matching only;
positive tests pin benign flags (`--offline`, `-- --test-threads=1`).

No concurrency, unsafe, or hostile-binary-input hazards in this change.
