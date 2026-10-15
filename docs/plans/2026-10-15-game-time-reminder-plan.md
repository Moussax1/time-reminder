# In-Game Time Reminder Utility — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a lightweight native Windows utility in Rust that detects when full-screen games are running and displays an unobtrusive in-game time reminder using the approved 8-bit monochrome Split Pod Card layout.

**Architecture:** A standalone background daemon that uses native Win32 APIs (`SHQueryUserNotificationState` for game detection, a click-through layered transparent window for the OSD overlay, and a system tray icon with an 8-bit GUI config dialog). The overlay never steals input focus or modifies game memory, making it 100% anti-cheat safe with under 5 MB RAM usage.

**Tech Stack:** Rust (stable MSVC toolchain), `windows` crate (Win32 APIs: User32, Gdi32, Shell32), `serde` / `serde_json`.

**Spec:** [docs/specs/2026-10-15-game-time-reminder-design.md](file:///c:/Users/yasso/Desktop/TIME%20REMINDER/docs/specs/2026-10-15-game-time-reminder-design.md)

## Global Constraints
- Target OS: Windows (x86_64 native binary).
- Input safety: Extended styles `WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TOOLWINDOW` must be used so mouse/keyboard inputs are never blocked or stolen from the game.
- Memory & Performance floor: < 5 MB RAM, 0.0% CPU when idle.
- Typography: Authentic 8-bit arcade pixel font (matching the user's reference image).
- Layout: Split Pod Card with 16px rounded outer corners, inner charcoal pod with 12px radius, pure monochrome black & white.

## Review Focus
1. **Game focus changes:** If a game is minimized (Alt-Tab), the session timer must pause without resetting active playtime.
2. **Click-through guarantee:** Mouse clicks over the notification area must pass directly through to whatever game control or crosshair is behind it.
3. **Invalid config recovery:** If `config.json` is corrupted or edited with invalid values, it must fall back to safe defaults without crashing.
4. **Resolution changes / DPI scaling:** The overlay must calculate position using the primary monitor's actual work area so it does not spawn off-screen.
5. **Clean tray exit:** Closing from the tray menu or settings window must cleanly unregister hotkeys and remove the tray icon from the Windows taskbar.

---

### Task 1: Cargo Project Setup & Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Test: `tests/setup_test.rs`

**Interfaces:**
- Produces: Base Cargo package `time-reminder` with required Windows API and Serde dependencies.

- [ ] **Step 1: Write test verifying environment and compilation**

```rust
// tests/setup_test.rs
#[test]
fn test_project_compiles() {
    assert!(true);
}
```

- [ ] **Step 2: Create Cargo.toml with necessary dependencies**

```toml
[package]
name = "time-reminder"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dependencies.windows]
version = "0.58"
features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_Shell",
    "Win32_Graphics_Gdi",
    "Win32_System_LibraryLoader",
]

[[bin]]
name = "time-reminder"
path = "src/main.rs"
```

- [ ] **Step 3: Create minimal src/main.rs**

```rust
fn main() {
    println!("Time Reminder daemon initialized.");
}
```

- [ ] **Step 4: Run cargo test to verify setup**
Run: `cmd /c cargo test`  
Expected: PASS

---

### Task 2: Configuration Module (`src/config.rs`)

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`
- Test: `tests/config_test.rs`

**Interfaces:**
- Produces: `Config` struct, `load_config() -> Config`, `save_config(&Config) -> Result<(), ...>`.

- [ ] **Step 1: Write failing config unit tests**

```rust
// tests/config_test.rs
use std::fs;

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
}
```

- [ ] **Step 2: Implement `src/config.rs`**

```rust
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
```

- [ ] **Step 3: Run tests to verify config passes**
Run: `cmd /c cargo test --test config_test`  
Expected: PASS

---

### Task 3: Game Detection & Session Tracker (`src/detector.rs`)

**Files:**
- Create: `src/detector.rs`
- Modify: `src/lib.rs` / `src/main.rs`
- Test: `tests/detector_test.rs`

**Interfaces:**
- Produces: `is_game_fullscreen() -> bool`, `SessionTracker` struct with `update(is_active) -> Duration`, `format_session(Duration) -> String`.

- [ ] **Step 1: Write failing session timer tests**

```rust
// tests/detector_test.rs
use std::time::Duration;

#[test]
fn test_session_formatting() {
    let d = Duration::from_secs(3600 + 30 * 60); // 1h 30m
    assert_eq!(time_reminder::detector::format_session(d), "01H 30M");
}

#[test]
fn test_session_tracker_pause_resume() {
    let mut tracker = time_reminder::detector::SessionTracker::new();
    tracker.tick(true, Duration::from_secs(60));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 60);
    tracker.tick(false, Duration::from_secs(120)); // paused
    assert_eq!(tracker.elapsed_playtime().as_secs(), 60);
}
```

- [ ] **Step 2: Implement detector and Win32 game query**

```rust
// src/detector.rs
use std::time::Duration;
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct SessionTracker {
    elapsed: Duration,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self { elapsed: Duration::ZERO }
    }

    pub fn tick(&mut self, is_playing: bool, delta: Duration) {
        if is_playing {
            self.elapsed += delta;
        }
    }

    pub fn elapsed_playtime(&self) -> Duration {
        self.elapsed
    }

    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
    }
}

pub fn format_session(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    format!("{:02}H {:02}M", hours, mins)
}

pub fn is_game_fullscreen() -> bool {
    unsafe {
        // Method 1: Shell32 D3D Fullscreen notification state
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_BUSY {
                return true;
            }
        }

        // Method 2: Fallback check on foreground window bounds matching screen
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            let mut rect = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let screen_w = GetSystemMetrics(SM_CXSCREEN);
                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                if rect.left <= 0 && rect.top <= 0 && rect.right >= screen_w && rect.bottom >= screen_h {
                    return true;
                }
            }
        }
        false
    }
}
```

- [ ] **Step 3: Run cargo test to verify detector logic**
Run: `cmd /c cargo test --test detector_test`  
Expected: PASS

---

### Task 4: 8-Bit Pixel Font & Split Pod Renderer (`src/renderer.rs`)

**Files:**
- Create: `src/renderer.rs`
- Create: `src/font_8bit.rs` (embedded 8x8 bitmap font array)
- Test: `tests/renderer_test.rs`

**Interfaces:**
- Produces: `draw_split_pod_card(hdc, width, height, time_str, ampm_str, session_str, cursor_blink)`

- [ ] **Step 1: Implement embedded 8x8 pixel font table in `src/font_8bit.rs`**
  Includes ASCII characters 0-9, A-Z, colon, hyphen, arrow cursor `►`. Each glyph is an 8-byte array (8x8 bitmap), scalable with clean integer factors (2x, 3x).

- [ ] **Step 2: Implement GDI Split Pod Card drawing in `src/renderer.rs`**
  - Draws outer pure black rounded rectangle with 2px white border (`radius: 16px`).
  - Draws left charcoal pod (`#141414`) with 12px radius.
  - Draws pixel text: `► 10:45 PM` in left pod.
  - Draws `SESSION 01H 30M` in right pod.

- [ ] **Step 3: Write test verifying string time parsing & formatting**

```rust
// tests/renderer_test.rs
#[test]
fn test_clock_formatting_12h() {
    let (time_str, ampm) = time_reminder::renderer::format_clock(14, 30, "12h");
    assert_eq!(time_str, "02:30");
    assert_eq!(ampm, "PM");
}

#[test]
fn test_clock_formatting_24h() {
    let (time_str, ampm) = time_reminder::renderer::format_clock(14, 30, "24h");
    assert_eq!(time_str, "14:30");
    assert_eq!(ampm, "");
}
```

- [ ] **Step 4: Run cargo test to verify renderer logic**
Run: `cmd /c cargo test --test renderer_test`  
Expected: PASS

---

### Task 5: Win32 Layered Overlay Window (`src/overlay.rs`)

**Files:**
- Create: `src/overlay.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `OverlayWindow` struct with `create()`, `trigger(time_str, ampm, session_str)`, `update_animation()`, `hide()`.

- [ ] **Step 1: Implement `OverlayWindow` using Win32 API**
  - Window class registered with `CS_HREDRAW | CS_VREDRAW`.
  - Window created with styles:
    `WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TOOLWINDOW`.
  - Alpha animation:
    - 0% to 100% over 200ms
    - Hold 100% for `display_duration_secs`
    - 100% to 0% over 300ms
    - Uses `SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA)`.
  - Positioning: Computes coordinates using `GetSystemMetrics` for `top-right`, `top-center`, `top-left`.

- [ ] **Step 2: Add runnable test/demo mode to flash overlay for 4 seconds**
  Run: `cmd /c cargo run -- --test-overlay`  
  Expected: Rounded Split Pod Card flashes in the top-right corner on the user's actual screen for 4 seconds, then smoothly disappears.

---

### Task 6: Retro 8-Bit Settings Window (`src/settings_gui.rs`)

**Files:**
- Create: `src/settings_gui.rs`

**Interfaces:**
- Produces: `open_settings_window(current_config, on_save_callback)`

- [ ] **Step 1: Implement 8-bit retro settings modal window**
  - Matches `retro-settings-window.png`:
    - 3px white border with 16px rounded corners.
    - Clickable buttons for:
      - `[ 15M ] [ 30M ✓ ] [ 45M ] [ 60M ]`
      - `[ TOP-RIGHT ✓ ] [ TOP-CENTER ] [ TOP-LEFT ]`
      - `[ 12-HOUR (AM/PM) ✓ ] [ 24-HOUR ]`
      - `[ SAVE CONFIG ]`
  - Mouse click handling: hit-tests the button rectangles, updates highlighted state, and calls callback with updated `Config`.

- [ ] **Step 2: Test opening settings window**
  Run: `cmd /c cargo run -- --settings`  
  Expected: Settings window appears, clicking buttons toggles selection, clicking "SAVE CONFIG" persists changes to `config.json`.

---

### Task 7: System Tray & Main Application Loop (`src/tray.rs` & `src/main.rs`)

**Files:**
- Create: `src/tray.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: Main daemon loop with `RegisterHotKey(Alt+T)`, `Shell_NotifyIconW`, right-click tray menu, and periodic game detection timer.

- [ ] **Step 1: Implement system tray icon & menu in `src/tray.rs`**
  - Uses `NOTIFYICONDATAW` with custom message `WM_USER + 1`.
  - On right-click: calls `CreatePopupMenu()`, adds checkmarked intervals (`15m`, `30m`, `45m`, `60m`), `Settings...`, `Test Notification`, and `Exit`.
  - On menu selection: dispatches actions immediately.

- [ ] **Step 2: Implement main thread and background timer in `src/main.rs`**
  - Registers global hotkey `Alt + T` via `RegisterHotKey`.
  - Timer loop ticks every 1 second:
    - Calls `is_game_fullscreen()`.
    - Updates `SessionTracker`.
    - If interval threshold elapsed while in game: triggers `OverlayWindow::trigger(...)`.
    - If `Alt + T` hotkey received: triggers `OverlayWindow::trigger(...)` immediately.
  - Clean shutdown handler on `WM_DESTROY` / tray Exit.

---

### Task 8: End-to-End Build & Verification

**Files:**
- Verify: Entire application
- Output: `target/release/time-reminder.exe`

- [ ] **Step 1: Run complete test suite**
Run: `cmd /c cargo test`  
Expected: All unit tests PASS.

- [ ] **Step 2: Build release executable**
Run: `cmd /c cargo build --release`  
Expected: Standalone binary `target/release/time-reminder.exe` generated (<3 MB).

- [ ] **Step 3: Verification checks**
  1. Trigger overlay via `Alt + T`: verify Split Pod Card renders cleanly with rounded corners and sharp 8-bit pixel font.
  2. Open settings window: verify options can be clicked and saved.
  3. Verify system tray right-click menu works and updates `config.json`.
  4. Verify memory usage in Task Manager is < 5 MB RAM.
