# In-Game Time Reminder Utility — Design Specification

**Date:** 2026-10-15  
**Language:** Rust (Windows native)  
**Status:** Validated & Ready for Review  

---

## 1. Overview & Objective
When playing immersive PC games, gamers frequently lose track of real-world time. This project is a dedicated, ultra-lightweight Windows utility built in **Rust** that runs silently in the background, detects when a game is running, and displays an unobtrusive in-game time reminder at configurable intervals (or on-demand via a global hotkey).

### Visual Aesthetic & Identity
- **Layout:** Split Pod Card with rounded corners (radius 16px, `rounded-2xl`). Two distinct compartments: a dedicated dark pod on the left for the bold 8-bit digital clock (`10:45 PM`), and an adjacent compartment on the right for active session playtime (`SESSION: 01H 30M`).
- **Typography:** Authentic 8-bit arcade pixel font (matching retro arcade / NES bitmap style from user reference).
- **Color Palette:** Pure Monochrome (High-contrast Pure Black & White).
  - Background: Pure Black (`#000000` / `#0a0a0a` at ~96% opacity).
  - Inner Clock Pod: Deep charcoal (`#141414`) with a crisp 1px white border.
  - Text & Accents: Pure White (`#ffffff`), blinking pixel arrow `►`.
  - Secondary Labels: Crisp muted gray (`#888888`).
- **HUD Card Shape:** Rounded corners (radius ~16px) with a crisp 2px white outer border.
- **Content:**
  - Current Local Clock Time (e.g., `10:45 PM` or `22:45`).
  - Active Game Session Playtime (e.g., `SESSION: 01H 30M`).

### Selected Prototype 1: In-Game Notification (Split Pod Card)

![Split Pod Card Prototype](assets/split-pod-card.png)

- **Dimensions:** ~260px width × 65px height.
- **Outer Card:** Pure black background (`#000000`), 2px solid white border, rounded corners (radius 16px).
- **Left Clock Pod:** Deep charcoal background (`#141414`), 1px subtle white border, rounded corners (radius 12px), white blinking pixel cursor `►`, bold pixel time `10:45`, muted `PM`.
- **Right Session Pod:** Muted gray `SESSION` label with bold white session playtime `01H 30M`.

---

### Selected Prototype 2: Retro 8-Bit Settings Window

![Retro Settings Window](assets/retro-settings-window.png)

- **Frame:** Authentic 8-bit arcade window with 3px solid white border and rounded corners (radius 16px).
- **Interactive Selectors:**
  - `REMINDER INTERVAL:` Clickable buttons for `[ 15M ]`, `[ 30M ✓ ]`, `[ 45M ]`, `[ 60M ]`.
  - `SCREEN POSITION:` Clickable buttons for `[ TOP-RIGHT ]`, `[ TOP-CENTER ]`, `[ TOP-LEFT ]`.
  - `CLOCK FORMAT:` Clickable buttons for `[ 12-HOUR (AM/PM) ]`, `[ 24-HOUR (MILITARY) ]`.
- **Actions:** White button `[ SAVE CONFIG ]`.

---

## 2. Architecture & Modules

```
┌────────────────────────────────────────────────────────┐
│                   Game Time Reminder                   │
├──────────────────────────┬─────────────────────────────┤
│      Detector Module     │        Overlay Module       │
│  SHQueryUserNotification │  Topmost Layered Window     │
│  Active Game Detection   │  8-Bit Font + Rounded Card  │
│  Session Timer Tracking  │  Smooth Alpha Fade In/Out   │
├──────────────────────────┼─────────────────────────────┤
│       Config Module      │         Tray Module         │
│  config.json (15-60 min) │  Windows System Tray Icon   │
│  Hotkeys & Preferences   │  Right-Click Context Menu   │
└──────────────────────────┴─────────────────────────────┘
```

### Module Breakdown
1. **`detector` (Game State & Session Tracker):**
   - Calls Windows API `SHQueryUserNotificationState` to determine if a full-screen DirectX/D3D/Vulkan game is currently active (`QUNS_RUNNING_D3D_FULL_SCREEN` / `QUNS_BUSY`).
   - Fallback: Checks foreground window geometry (`GetForegroundWindow` + `GetWindowRect`) against display bounds.
   - Maintains an active game session timer: starts counting when a game enters foreground, pauses when game loses focus or closes.

2. **`overlay` (On-Screen Display Window):**
   - Win32 layered click-through window with extended window styles:
     - `WS_EX_TOPMOST`: Always visible above the game.
     - `WS_EX_TRANSPARENT`: Total mouse click-through (clicks pass directly to game without delay).
     - `WS_EX_NOACTIVATE`: Never steals keyboard or window focus from the game.
     - `WS_EX_TOOLWINDOW`: Hidden from taskbar and Alt+Tab switcher.
     - `WS_EX_LAYERED`: Per-pixel alpha blending for smooth fade-in (200ms) and fade-out (300ms).
   - Renders the rounded-corner card with the 8-bit pixel font and blinking cursor.

3. **`tray` (System Tray & Background Daemon):**
   - Standard Windows Notification Area (System Tray) icon.
   - Right-click quick context menu:
     - Interval selection: `Every 15 min`, `Every 30 min (default)`, `Every 45 min`, `Every 60 min`.
     - `Settings...` (Opens the Retro 8-Bit Settings Window).
     - `Test Notification (Alt+T)`.
     - `Reset Session Timer`.
     - `Exit`.

4. **`settings_gui` (Retro 8-Bit Config Window):**
   - Dedicated retro 8-bit modal window opened via tray menu or double-click.
   - Interactive clickable button selectors for:
     - Reminder interval: `[15m] [30m] [45m] [60m]`
     - Screen position: `[Top-Right] [Top-Center] [Top-Left]`
     - Clock format: `[12-Hour (AM/PM)] [24-Hour]`
     - Save & Close button.

5. **`config` (Settings Persistence):**
   - Stores user preferences in `config.json` next to the executable:
     ```json
     {
       "interval_minutes": 30,
       "display_duration_secs": 4,
       "hotkey": "Alt+T",
       "time_format": "12h",
       "position": "top-right",
       "rounded_radius": 16
     }
     ```
   - Automatically reloads changes if `config.json` is edited directly.

---

## 3. Interaction & Input Safety
- **Anti-Cheat Safe:** Does NOT inject DLLs, hook DirectX/Vulkan pipelines, or modify game memory. It is purely an external, non-interactive Windows layered window.
- **Game Control Isolation:** Using `WS_EX_NOACTIVATE` guarantees that the game retains 100% of mouse and keyboard inputs at all times, even while the reminder animates on screen.

---

## 4. Performance & Resource Footprint
- Idle CPU usage: `0.0%`.
- Memory usage: `< 5 MB RAM`.
- Standalone executable size: `< 3 MB` (zero external runtimes required).

---

## 5. Verification & Testing Plan
1. **Unit & Logic Tests:** Config serialization/deserialization, time formatting (12h/24h), session timer accumulation.
2. **Display & Overlay Verification:** Trigger overlay with `Alt+T` and confirm:
   - Rounded corners render correctly.
   - 8-bit pixel font is sharp and legible.
   - No focus stealing or mouse blocking.
3. **Game Detection Verification:** Launch a fullscreen game / window and confirm automatic interval trigger.
