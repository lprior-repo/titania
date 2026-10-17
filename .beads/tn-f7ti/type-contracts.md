# Type contracts — tn-f7ti

- Registry { version: u32, targets: Vec<RegistryEntry> } — parsed boundary
  type; every constructor path validates (empty path, duplicate path,
  unknown field, misplaced authority all rejected).
- RegistryEntry { path: String, authority: RegistryAuthority } — path is the
  stable obligation identity; authority defaults to production.
- SourceClass { GenuineProof, SmokeOnly, ProofFree } — content verdict, never
  derived from extension.
- AuditFailure — six variants, each naming its obligation or path; Display
  text is the consumer-visible reason.
- AuditReport { passed, failures } — is_clean is true only with zero
  failures; passed preserves registry order.
- ReadError { path, kind: NotFound | Io } — missing and unreadable stay
  distinct so the audit can name absence separately.
- WorkspaceAuditError — registry unreadable/invalid and source-root
  unreadable/unlistable stay distinct.
- audit() is pure: file access enters only through an injected reader, so
  every behavior check runs without filesystem or clock.
