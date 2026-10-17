# Error taxonomy — tn-vr2n

Railway (core): `InvocationError::CapLints { arg }` — domain error via
thiserror; message names the offending token and states the consequence
(lint severity would be capped below the mandatory denies).

Shell mapping: `RunCargoError::Policy(String)` → `LaneExit::Violations`
(exit 1, Reject per v1-spec §12) with the message on stderr. Chosen over
`Usage` (exit 3, input error) because a ceiling bypass is a rejected
verdict, not a CLI usage mistake; chosen over `Failure` (exit ≥4,
infra/internal) because the runner itself is healthy.

No `Result<T, String>` in core; no swallowed errors; every fallible
spawn/join unchanged.
