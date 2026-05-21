use xtask::evidence::{
    GateProfile, UiReleaseToolingLane, UiReleaseToolingLaneKind, ui_release_tooling_lanes,
};

#[test]
fn formal_tooling_lanes_are_publicly_classified_release_evidence() {
    let lanes = ui_release_tooling_lanes();

    assert_eq!(lanes.len(), 7);
    assert_machine_gate(lane(lanes, "kani-inventory"));
    assert_machine_gate(lane(lanes, "kani-layout-predicates"));
    assert_machine_gate(lane(lanes, "redaction-fuzz"));
    assert_machine_gate(lane(lanes, "moon-ci"));
    assert_executable_gate(lane(lanes, "miri"));
    assert_executable_gate(lane(lanes, "mutants"));
    assert_executable_gate(lane(lanes, "coverage"));
}

#[test]
fn executable_tooling_lanes_map_to_public_gate_profiles() {
    let deep_gates = GateProfile::AiDeep.gates();
    let release_gates = GateProfile::AiRelease.gates();

    assert!(deep_gates.contains(&"miri"));
    assert!(deep_gates.contains(&"mutants"));
    assert!(deep_gates.contains(&"llvm-cov"));
    assert!(release_gates.contains(&"miri"));
    assert!(release_gates.contains(&"coverage"));
    assert!(release_gates.contains(&"mutants-smoke"));
}

fn lane<'a>(lanes: &'a [UiReleaseToolingLane], name: &str) -> &'a UiReleaseToolingLane {
    static FALLBACK_LANE: UiReleaseToolingLane = UiReleaseToolingLane {
        name: "missing",
        command: "missing",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("missing test fallback"),
    };

    match lanes.iter().find(|lane| lane.name == name) {
        Some(lane) => lane,
        None => {
            assert_eq!(name, "", "missing tooling lane: {name}");
            &FALLBACK_LANE
        }
    }
}

fn assert_machine_gate(lane: &UiReleaseToolingLane) {
    assert_eq!(lane.kind, UiReleaseToolingLaneKind::ExternalMachineGate);
    assert!(lane.blocker.is_some());
    assert!(!lane.command.is_empty());
}

fn assert_executable_gate(lane: &UiReleaseToolingLane) {
    assert_eq!(lane.kind, UiReleaseToolingLaneKind::ExecutableGate);
    assert_eq!(lane.blocker, None);
    assert!(!lane.command.is_empty());
}
