# 🕹️ Game Time Reminder (Windows Native)

An ultra-lightweight, zero-distraction in-game time reminder tool built in pure Rust with direct Win32 API integration. Periodically displays a crisp, retro 8-bit Split Pod Card showing the current digital time and active session playtime so you never lose track of time while gaming.

![Split Pod Card](docs/specs/assets/split-pod-card.png)

---

## ⚡ Highlights

- **Authentic 8-Bit Retro Aesthetic**: Uses custom embedded 8-bit arcade bitmap glyphs rendered on a curved **Split Pod Card** with pure monochrome black & white contrast.
- **Ultra-Lightweight & Zero-Lag**:
  - Memory footprint: **< 3 MB RAM**.
  - CPU usage: **0.0% idle**.
  - Binary size: **~1.49 MB** standalone `.exe` (zero runtimes, zero DLL dependencies).
- **Anti-Cheat Safe & Transparent**: Uses `WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_LAYERED`. Clicks pass directly through into your game without stealing window focus or interrupting input.
- **Smart Game Detection**: Uses Win32 `SHQueryUserNotificationState` (Direct3D fullscreen state) + foreground bounds check. Only counts active playtime when your game is actually in the foreground.
- **Dual Configuration UX**:
  1. **System Tray Menu**: Right-click the clock icon in the Windows taskbar tray to switch reminder intervals (15m, 30m, 45m, 60m) in 1 click, trigger a test notification, or reset session time.
  2. **Retro 8-Bit Settings GUI**: Visual modal window to toggle interval, position, and 12h/24h format with interactive retro buttons.

---

## 🚀 Quick Start

### 1. Launching the App
Simply double-click `TimeReminder.bat` or run:
```cmd
target\release\time-reminder.exe
```
When started, the daemon flashes the reminder card once in the top-right corner to confirm it is active, then sits silently in the system tray.

### 2. Hotkeys & Controls
- **`Alt + T`**: Instantly flashes the reminder card on-screen anytime (even during a game).
- **Right-Click System Tray Icon**:
  - `[✓] 30m` (Interval selection)
  - `Settings...` (Opens the retro 8-bit GUI)
  - `Test Notification` (Flashes the card)
  - `Reset Session Time` (Zeroes out the session counter)
  - `Exit` (Quits cleanly)

### 3. CLI Helper Commands
- `target\release\time-reminder.exe --test-overlay`: Flashes the overlay for 4 seconds and exits.
- `target\release\time-reminder.exe --settings`: Opens the retro settings dialog directly.

---

## ⚙️ Configuration (`config.json`)

Settings are saved in `config.json` next to the executable:
```json
{
  "interval_minutes": 30,
  "display_duration_secs": 4,
  "position": "top-right",
  "time_format": "12h",
  "theme": "monochrome"
}
```

| Key | Values | Description |
|---|---|---|
| `interval_minutes` | `15`, `30`, `45`, `60` | Playtime duration between reminders |
| `display_duration_secs` | Integer (e.g. `4`) | Number of seconds overlay stays visible |
| `position` | `"top-right"`, `"top-center"`, `"top-left"` | Screen anchor location |
| `time_format` | `"12h"`, `"24h"` | 12-hour (with AM/PM) or 24-hour clock |
| `theme` | `"monochrome"` | Pure black & white high-contrast card |

---

## 🛠️ Building from Source

Requirements:
- Rust toolchain (e.g., `stable-x86_64-pc-windows-gnu` or `stable-x86_64-pc-windows-msvc`)

```cmd
cargo test
cargo build --release
```
The compiled standalone binary is placed in `target/release/time-reminder.exe`.
