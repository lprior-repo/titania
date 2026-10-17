use titania_core::MutantId;

#[derive(Debug, Default)]
pub(super) struct MutantsReport {
    pub(super) survivor_names: Vec<String>,
    pub(super) exit_code: i32,
}

#[derive(Debug, Default)]
pub(super) struct LaneRunState {
    pub(super) tool_version: String,
    pub(super) cgroup_used: bool,
}

#[derive(Debug, Clone)]
pub(super) struct NewSurvivor {
    pub(super) package: String,
    pub(super) rel_path: String,
    pub(super) line: u32,
    pub(super) column: u32,
    pub(super) genre: String,
    pub(super) replacement: String,
    pub(super) raw_name: String,
    pub(super) typed_id: MutantId,
}
