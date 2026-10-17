# Domain model — tn-f7ti

Ubiquitous language: obligation (one registry row), authority (production
vs smoke-fixture), source class (genuine proof, smoke-only, proof-free),
inventory (one audit run), disposition (recorded acceptance of an
unregistered production proof), stable identity (workspace-relative path).

Entities: Registry (versioned list of RegistryEntry rows). Value objects:
RegistryAuthority, SourceClass, AuditFailure (six named variants),
AuditReport (passed identities plus named failures). Aggregate boundary:
the registry document plus the file set it governs; required production
obligations are the protected subset that must stay registered.

Invariants: unknown registry sentences are rejected, never ignored; the
smoke tag wins over proof tokens; extension alone proves nothing; comment
and string content proves nothing; over-stripping fails loud, never silent.
