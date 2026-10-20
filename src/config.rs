use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_KEY_SUBKEY: windows::core::PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const APP_REG_NAME: windows::core::PCWSTR = w!("TimeReminder");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub interval_minutes: u32,
    pub display_duration_secs: u32,
    pub time_format: String, // "12h" or "24h"
    pub position: String,    // "top-right", "top-center", "top-left"
    pub hotkey: String,      // "Alt+T"
    #[serde(default)]
    pub run_on_startup: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_minutes: 30,
            display_duration_secs: 4,
            time_format: "12h".to_string(),
            position: "top-right".to_string(),
            hotkey: "Alt+T".to_string(),
            run_on_startup: false,
        }
    }
}

pub fn set_run_on_startup(enable: bool) -> Result<(), String> {
    unsafe {
        let mut hkey = HKEY::default();
        let res = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY_SUBKEY,
            0,
            KEY_WRITE,
            &mut hkey,
        );
        if res.is_err() {
            return Err("Failed to open HKCU Run registry key for writing".to_string());
        }

        if enable {
            let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
            let formatted_path = format!("\"{}\"", exe_path.display());
            let wide_val: Vec<u16> = formatted_path.encode_utf16().chain(std::iter::once(0)).collect();
            let slice = std::slice::from_raw_parts(
                wide_val.as_ptr() as *const u8,
                wide_val.len() * 2,
            );
            let set_res = RegSetValueExW(
                hkey,
                APP_REG_NAME,
                0,
                REG_SZ,
                Some(slice),
            );
            let _ = RegCloseKey(hkey);
            if set_res.is_err() {
                return Err("Failed to write to Run registry key".to_string());
            }
        } else {
            let _ = RegDeleteValueW(hkey, APP_REG_NAME);
            let _ = RegCloseKey(hkey);
        }
        Ok(())
    }
}

pub fn check_run_on_startup() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY_SUBKEY,
            0,
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }

        let mut val_type = windows::Win32::System::Registry::REG_VALUE_TYPE(0);
        let mut data_len = 0u32;
        let query_res = RegQueryValueExW(
            hkey,
            APP_REG_NAME,
            None,
            Some(&mut val_type),
            None,
            Some(&mut data_len),
        );
        let _ = RegCloseKey(hkey);
        query_res.is_ok() && data_len > 0
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
    let p = path.as_ref();
    let tmp = p.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, p)
}

/// Parses hotkey strings like "Alt+T", "Ctrl+Shift+T" into (modifiers, vk_code).
/// Fallbacks to (MOD_ALT, 0x54 for 'T') if unparseable.
pub fn parse_hotkey(hotkey: &str) -> (u32, u32) {
    let mut modifiers = 0u32;
    let mut vk = 0x54u32; // Default 'T'
    for part in hotkey.split('+') {
        let part = part.trim();
        if part.eq_ignore_ascii_case("alt") {
            modifiers |= 0x0001; // MOD_ALT
        } else if part.eq_ignore_ascii_case("ctrl") || part.eq_ignore_ascii_case("control") {
            modifiers |= 0x0002; // MOD_CONTROL
        } else if part.eq_ignore_ascii_case("shift") {
            modifiers |= 0x0004; // MOD_SHIFT
        } else if part.eq_ignore_ascii_case("win") {
            modifiers |= 0x0008; // MOD_WIN
        } else if part.len() == 1 {
            let c = part.chars().next().unwrap().to_ascii_uppercase();
            vk = c as u32;
        }
    }
    if modifiers == 0 {
        modifiers = 0x0001; // Default to MOD_ALT
    }
    (modifiers, vk)
}

