#[test]
fn test_project_compiles_and_version_matches() {
    assert_eq!(time_reminder::version(), "0.1.0");
}

#[test]
fn test_main_contains_windows_subsystem_attribute() {
    let main_src = include_str!("../src/main.rs");
    assert!(
        main_src.contains(r#"#![windows_subsystem = "windows"]"#),
        "src/main.rs must specify windows_subsystem = \"windows\" to prevent console window popup"
    );
}
