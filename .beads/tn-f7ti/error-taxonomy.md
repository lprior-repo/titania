# Error taxonomy — tn-f7ti

Registry boundary (RegistryError): MissingVersion, InvalidVersion,
MissingTargets, UnexpectedLine, EmptyPath, DuplicatePath,
InvalidAuthority, MisplacedAuthority. All fail the read; none degrade to a
warning or a default row.

Audit verdicts (AuditFailure): RegisteredMissing, RegisteredUnreadable,
RegisteredProofFree, RegisteredSmokeAsProduction,
RequiredObligationRemoved, UnregisteredProductionProof. Each names its
obligation or path in both the variant payload and the Display text.

Read boundary (ReadErrorKind): NotFound vs Io — absence and unreadability
are different consumer-visible facts and stay separate through the report.

Workspace boundary (WorkspaceAuditError): RegistryUnreadable (with I/O
detail), RegistryInvalid (transparent parse error), SourceRootUnreadable
(with detail), SourceListingFailed (with detail).

No error is swallowed: every fallible read, listing entry, and parse line
maps to a typed variant or a pass.
