#[test]
fn test_default_config_values() {
    let cfg = time_reminder::config::Config::default();
    assert_eq!(cfg.interval_minutes, 30);
    assert_eq!(cfg.display_duration_secs, 4);
    assert_eq!(cfg.time_format, "12h");
    assert_eq!(cfg.position, "top-right");
    assert!(!cfg.run_on_startup);
}

#[test]
fn test_config_serialization_roundtrip() {
    let cfg = time_reminder::config::Config {
        run_on_startup: true,
        ..Default::default()
    };
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let loaded: time_reminder::config::Config = serde_json::from_str(&json).unwrap();
    assert_eq!(cfg.interval_minutes, loaded.interval_minutes);
    assert_eq!(cfg.position, loaded.position);
    assert!(loaded.run_on_startup);
}

#[test]
fn test_run_on_startup_backward_compatibility() {
    // Legacy JSON without run_on_startup key
    let legacy_json = r#"{
        "interval_minutes": 15,
        "display_duration_secs": 4,
        "time_format": "24h",
        "position": "top-center",
        "hotkey": "Alt+T"
    }"#;
    let loaded: time_reminder::config::Config = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(loaded.interval_minutes, 15);
    assert!(!loaded.run_on_startup);
}

#[test]
fn test_run_on_startup_registry_functions() {
    use time_reminder::config::{check_run_on_startup, set_run_on_startup};
    let initial = check_run_on_startup();

    // Verify enabling
    let res = set_run_on_startup(true);
    assert!(res.is_ok());
    assert!(check_run_on_startup());

    // Verify disabling
    let res_disable = set_run_on_startup(false);
    assert!(res_disable.is_ok());
    assert!(!check_run_on_startup());

    // Restore initial state
    let _ = set_run_on_startup(initial);
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

#[test]
fn test_parse_hotkey() {
    use time_reminder::config::parse_hotkey;
    // Default Alt+T
    let (mod_flags, vk) = parse_hotkey("Alt+T");
    assert_eq!(mod_flags, 0x0001); // MOD_ALT
    assert_eq!(vk, 0x54); // 'T'

    // Ctrl+Shift+G
    let (mod_flags, vk) = parse_hotkey("Ctrl+Shift+G");
    assert_eq!(mod_flags, 0x0002 | 0x0004); // MOD_CONTROL | MOD_SHIFT
    assert_eq!(vk, 0x47); // 'G'
}

#[test]
fn test_atomic_save_config() {
    let temp_path = std::env::temp_dir().join("test_atomic_config.json");
    let cfg = time_reminder::config::Config {
        interval_minutes: 45,
        ..Default::default()
    };
    let res = time_reminder::config::save_config(&temp_path, &cfg);
    assert!(res.is_ok());
    let loaded = time_reminder::config::load_config(&temp_path);
    assert_eq!(loaded.interval_minutes, 45);
    // Ensure no dangling .tmp file remains
    let tmp_path = temp_path.with_extension("tmp");
    assert!(!tmp_path.exists());
    let _ = std::fs::remove_file(&temp_path);
}
