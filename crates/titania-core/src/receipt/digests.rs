use crate::Digest;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceiptDigests {
    source: Digest,
    lock: Digest,
    policy: Digest,
    toolchain: Digest,
}

impl ReceiptDigests {
    #[must_use]
    pub const fn new(source: Digest, lock: Digest, policy: Digest, toolchain: Digest) -> Self {
        Self { source, lock, policy, toolchain }
    }

    #[must_use]
    pub const fn source(&self) -> &Digest {
        &self.source
    }

    #[must_use]
    pub const fn lock(&self) -> &Digest {
        &self.lock
    }

    #[must_use]
    pub const fn policy(&self) -> &Digest {
        &self.policy
    }

    #[must_use]
    pub const fn toolchain(&self) -> &Digest {
        &self.toolchain
    }

    pub(crate) fn into_parts(self) -> (Digest, Digest, Digest, Digest) {
        (self.source, self.lock, self.policy, self.toolchain)
    }
}
