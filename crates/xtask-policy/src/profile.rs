//! Policy profile types.

use serde::{Deserialize, Serialize};

use xtask_core::Digest;

/// A loaded quality policy profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyProfile {
    /// Profile name (e.g. "strict-ai").
    pub name: String,
    /// The raw policy TOML content.
    pub raw_toml: String,
    /// BLAKE3 digest of all policy files.
    pub policy_digest: Digest,
}
