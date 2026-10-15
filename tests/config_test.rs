#[test]
fn test_default_config_values() {
    let cfg = time_reminder::config::Config::default();
    assert_eq!(cfg.interval_minutes, 30);
    assert_eq!(cfg.display_duration_secs, 4);
    assert_eq!(cfg.time_format, "12h");
    assert_eq!(cfg.position, "top-right");
}

#[test]
fn test_config_serialization_roundtrip() {
    let cfg = time_reminder::config::Config::default();
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let loaded: time_reminder::config::Config = serde_json::from_str(&json).unwrap();
    assert_eq!(cfg.interval_minutes, loaded.interval_minutes);
    assert_eq!(cfg.position, loaded.position);
}

#[test]
fn test_load_config_fallback_when_missing() {
    let temp_path = std::env::temp_dir().join("test_nonexistent_time_reminder_config.json");
    let _ = std::fs::remove_file(&temp_path);
    let cfg = time_reminder::config::load_config(&temp_path);
    assert_eq!(cfg.interval_minutes, 30);
    assert!(temp_path.exists());
    let _ = std::fs::remove_file(&temp_path);
}
