# Workflow model — tn-f7ti

Inventory run order: read registry document → validate schema → read each
registered file → classify content → check authority fit → verify required
rows still registered → classify discovered candidates → report.

Transitions: a registered production file lands in exactly one of passed,
missing, unreadable, proof-free, or smoke-only. A required row absent from
the registry lands in removed-obligation failure regardless of all else. A
discovered production file with genuine proof lands in
unregistered-requires-disposition; discovered proof-free, smoke-only, or
unreadable files land nowhere.

No transition consults file extension for evidence; extension selects
discovery candidates only. No transition invents disposition: unregistered
proof always fails until a recorded disposition exists.
