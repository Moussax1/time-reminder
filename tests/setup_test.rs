#[test]
fn test_project_compiles_and_version_matches() {
    assert_eq!(time_reminder::version(), "0.1.0");
}
