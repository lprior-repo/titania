use titania_core::{Digest, RuleId, TextRange, WorkspacePath};

#[test]
fn scenario_receiver_pipeline_digest() {
    let payload = b"src/lib.rs:42: this is the source line we care about";
    let digest = Digest::from_bytes(payload);
    assert_eq!(digest.as_hex().len(), 64);
    assert!(digest.as_hex().bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
}

#[test]
fn scenario_rule_catalog_names_finding() {
    let raw = "func_loops_for";
    let id = RuleId::normalize(raw).unwrap();
    assert_eq!(id.as_str(), "FUNC_LOOPS_FOR");
    assert!(id.has_prefix("FUNC"));
    assert_eq!(id.to_string(), "FUNC_LOOPS_FOR");
}

#[test]
fn scenario_path_round_trip_and_reject_traversal() {
    let benign = WorkspacePath::new("src/parser.rs").unwrap();
    let malicious_attempts =
        ["../etc/passwd", "src/../../etc/shadow", "/etc/passwd", "ok\\..\\bad"];

    let json = serde_json::to_string(&benign).unwrap();
    let recovered: WorkspacePath = serde_json::from_str(&json).unwrap();

    assert_eq!(benign, recovered);
    assert_eq!(benign.as_str(), "src/parser.rs");
    for bad in malicious_attempts {
        assert!(WorkspacePath::new(bad).is_err(), "expected {bad} to be rejected");
    }
}

#[test]
fn scenario_text_range_contains_target_byte() {
    let r = TextRange::new(6, 11).unwrap();
    let inside = r.contains_byte(8);
    let before = r.contains_byte(3);
    let after = r.contains_byte(11);
    let empty_at_start = TextRange::new(6, 6).unwrap().contains_byte(6);
    assert!(inside);
    assert!(!before);
    assert!(!after);
    assert!(!empty_at_start);
    assert_eq!(r.width(), 5);
}
