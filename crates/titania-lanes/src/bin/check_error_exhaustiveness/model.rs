use std::{io::ErrorKind, path::PathBuf};

use titania_core::TargetProject;

#[derive(Clone, Copy)]
pub struct TargetRelativePath {
    value: &'static str,
}

impl TargetRelativePath {
    const fn new(value: &'static str) -> Self {
        Self { value }
    }

    pub const fn as_str(self) -> &'static str {
        self.value
    }

    pub fn in_target(self, target: &TargetProject) -> PathBuf {
        target.as_std_path().join(self.value)
    }
}

pub struct Oracle {
    pub path: TargetRelativePath,
    pub function: &'static str,
}

pub struct Check {
    pub type_name: &'static str,
    pub enum_path: TargetRelativePath,
    pub domain_label: &'static str,
    pub oracles: &'static [Oracle],
}

pub enum DomainFile {
    Present(String),
    Absent,
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
