use std::{io::ErrorKind, path::PathBuf};

use titania_core::TargetProject;

/// Path relative to the target project root.
#[derive(Debug, Clone, Copy)]
pub struct TargetRelativePath {
    value: &'static str,
}

impl TargetRelativePath {
    const fn new(value: &'static str) -> Self {
        Self { value }
    }

    /// Borrow the stored target-relative path string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.value
    }

    /// Resolve this path under a target project root.
    #[must_use]
    pub fn in_target(self, target: &TargetProject) -> PathBuf {
        target.as_std_path().join(self.value)
    }
}

/// Fuzz or test oracle function that must mention every error variant.
#[derive(Debug)]
pub struct Oracle {
    /// Oracle source file path.
    pub path: TargetRelativePath,
    /// Oracle function name.
    pub function: &'static str,
}

/// One error enum and its associated oracle functions.
#[derive(Debug)]
pub struct Check {
    /// Error enum type name.
    pub type_name: &'static str,
    /// Source path containing the error enum.
    pub enum_path: TargetRelativePath,
    /// Human-readable domain label for status output.
    pub domain_label: &'static str,
    /// Oracle functions expected to cover the enum variants.
    pub oracles: &'static [Oracle],
}

/// Optional domain file read result.
#[derive(Debug)]
pub enum DomainFile {
    /// File exists and was read as UTF-8 text.
    Present(String),
    /// File is absent, making the associated check not applicable.
    Absent,
    /// File exists but could not be read.
    Unreadable(ErrorKind),
}

const JOURNAL_ORACLES: &[Oracle] = &[
    Oracle {
        path: TargetRelativePath::new("fuzz/src/lib.rs"),
        function: "assert_typed_journal_error",
    },
    Oracle {
        path: TargetRelativePath::new("fuzz/fuzz_targets/decode_record.rs"),
        function: "assert_typed_journal_error",
    },
    Oracle {
        path: TargetRelativePath::new("fuzz/fuzz_targets/journal_decode.rs"),
        function: "assert_typed_journal_error",
    },
    Oracle {
        path: TargetRelativePath::new("fuzz/tests/proptest_journal_error_exhaustiveness.rs"),
        function: "assert_known_journal_error",
    },
];

const IPC_ORACLES: &[Oracle] = &[Oracle {
    path: TargetRelativePath::new("fuzz/src/lib.rs"),
    function: "assert_typed_ipc_error",
}];

const VALIDATION_ORACLES: &[Oracle] = &[Oracle {
    path: TargetRelativePath::new("fuzz/src/lib.rs"),
    function: "assert_typed_validation_error",
}];

/// Static error-exhaustiveness checks executed by this lane.
pub const CHECKS: &[Check] = &[
    Check {
        type_name: "JournalError",
        enum_path: TargetRelativePath::new("crates/vb_storage/src/error/mod.rs"),
        domain_label: "vb_storage",
        oracles: JOURNAL_ORACLES,
    },
    Check {
        type_name: "IpcError",
        enum_path: TargetRelativePath::new("crates/vb_ipc/src/error.rs"),
        domain_label: "vb_ipc",
        oracles: IPC_ORACLES,
    },
    Check {
        type_name: "ValidationError",
        enum_path: TargetRelativePath::new("crates/vb_validate/src/lib.rs"),
        domain_label: "vb_validate",
        oracles: VALIDATION_ORACLES,
    },
];
