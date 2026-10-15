use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub interval_minutes: u32,
    pub display_duration_secs: u32,
    pub time_format: String, // "12h" or "24h"
    pub position: String,    // "top-right", "top-center", "top-left"
    pub hotkey: String,      // "Alt+T"
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_minutes: 30,
            display_duration_secs: 4,
            time_format: "12h".to_string(),
            position: "top-right".to_string(),
            hotkey: "Alt+T".to_string(),
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Config {
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
            return cfg;
        }
    }
    let default = Config::default();
    let _ = save_config(&path, &default);
    default
}

pub fn save_config<P: AsRef<Path>>(path: P, cfg: &Config) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(path, json)
}
