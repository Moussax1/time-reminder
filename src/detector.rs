use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId,
    SM_CXSCREEN, SM_CYSCREEN,
};

static LAST_GAMEPAD_PACKETS: [AtomicU32; 4] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];
static GAMEPAD_SEEN: [AtomicBool; 4] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];
static LAST_GAMEPAD_TIME: AtomicU32 = AtomicU32::new(0);

/// Threshold of continuous user inactivity before pausing active gameplay tracking (3 minutes).
pub const IDLE_ACTIVE_THRESHOLD: Duration = Duration::from_secs(180);

/// Measures how long since the user last provided input (keyboard, mouse, or gamepad).
pub fn get_idle_duration() -> Duration {
    // Fail-closed default: if GetLastInputInfo fails, assume user is idle
    let mut kb_mouse_idle_ms = u32::MAX;
    // SAFETY: GetLastInputInfo safely populates the LASTINPUTINFO structure.
    // cbSize is zero-initialized and set to std::mem::size_of::<LASTINPUTINFO>().
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii).as_bool() {
            let now = GetTickCount();
            let diff = now.wrapping_sub(lii.dwTime);
            // Handle timer tick jitter / concurrent input between calls
            kb_mouse_idle_ms = if diff > (1 << 31) { 0 } else { diff };
        }
    }

    // SAFETY: GetTickCount is a side-effect-free Win32 system call returning uptime in milliseconds.
    let now = unsafe { GetTickCount() };

    let mut any_gamepad_connected = false;
    for user_index in 0..4 {
        let mut state = XINPUT_STATE::default();
        // SAFETY: XInputGetState is called with valid user controller indices (0..4)
        // and a valid pointer to a zeroed stack XINPUT_STATE struct.
        let res = unsafe { XInputGetState(user_index, &mut state) };
        let slot = user_index as usize;
        if res == 0 {
            any_gamepad_connected = true;
            if !GAMEPAD_SEEN[slot].load(Ordering::Relaxed) {
                // Controller just connected: capture initial packet without faking input
                LAST_GAMEPAD_PACKETS[slot].store(state.dwPacketNumber, Ordering::Relaxed);
                GAMEPAD_SEEN[slot].store(true, Ordering::Relaxed);
            } else {
                let last_packet = LAST_GAMEPAD_PACKETS[slot].load(Ordering::Relaxed);
                if state.dwPacketNumber != last_packet {
                    LAST_GAMEPAD_PACKETS[slot].store(state.dwPacketNumber, Ordering::Relaxed);
                    LAST_GAMEPAD_TIME.store(now, Ordering::Relaxed);
                }
            }
        } else {
            GAMEPAD_SEEN[slot].store(false, Ordering::Relaxed);
        }
    }

    let gamepad_idle_ms = if any_gamepad_connected {
        let last_gamepad = LAST_GAMEPAD_TIME.load(Ordering::Relaxed);
        if last_gamepad > 0 {
            let diff = now.wrapping_sub(last_gamepad);
            if diff > (1 << 31) {
                0
            } else {
                diff
            }
        } else {
            u32::MAX
        }
    } else {
        u32::MAX
    };

    let min_idle_ms = kb_mouse_idle_ms.min(gamepad_idle_ms);
    Duration::from_millis(min_idle_ms as u64)
}

pub fn is_user_input_active(max_idle: Duration) -> bool {
    get_idle_duration() < max_idle
}

/// Standard game library directory segments on Windows.
/// Detects games installed via Steam, Epic Games, Xbox GamePass, GOG, Riot, Ubisoft, EA, Battle.net, or any \Games\ directory.
pub const GAME_PATH_MARKERS: &[&str] = &[
    "\\steamapps\\",
    "\\epic games\\",
    "\\xboxgames\\",
    "\\windowsapps\\",
    "\\gog games\\",
    "\\gog galaxy\\games\\",
    "\\riot games\\",
    "\\ubisoft\\",
    "\\ea games\\",
    "\\origin games\\",
    "\\battle.net\\",
    "\\battlenet\\",
    "\\games\\",
];

/// Known UWP media/streaming package signatures under C:\Program Files\WindowsApps\
pub const UWP_MEDIA_EXCLUSIONS: &[&str] = &[
    "netflix",
    "microsoft.zunevideo",
    "microsoft.windowsmedia",
    "microsoft.media.player",
    "spotifyab.spotifymusic",
    "primevideo",
    "disneyplus",
    "hulu",
];

pub fn is_game_path(path: &str) -> bool {
    let lower = if path.bytes().any(|b| b.is_ascii_uppercase()) {
        std::borrow::Cow::Owned(path.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(path)
    };
    // Exclude UWP media and video players that reside inside WindowsApps
    if lower.contains("\\windowsapps\\") && UWP_MEDIA_EXCLUSIONS.iter().any(|ex| lower.contains(ex))
    {
        return false;
    }
    GAME_PATH_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

pub fn is_desktop_or_shell_window(class_name: &str) -> bool {
    matches!(
        class_name,
        "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
    )
}

pub fn get_window_class_name(hwnd: HWND) -> Option<String> {
    if hwnd.is_invalid() {
        return None;
    }
    // SAFETY: GetClassNameW writes up to buffer length null-terminated UTF-16 characters.
    unsafe {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buf);
        if len > 0 {
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        } else {
            None
        }
    }
}

pub fn get_process_info(pid: u32) -> Option<(String, String)> {
    if pid == 0 {
        return None;
    }
    // SAFETY: OpenProcess requests PROCESS_QUERY_LIMITED_INFORMATION, which allows querying process name and path.
    // Handle is cleanly closed with CloseHandle in all branches.
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let res = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            );
            let _ = CloseHandle(handle);
            if res.is_ok() && len > 0 {
                let full_path = String::from_utf16_lossy(&buf[..len as usize]);
                let file_name = std::path::Path::new(&full_path)
                    .file_name()?
                    .to_str()?
                    .to_lowercase();
                return Some((full_path, file_name));
            }
        }
        None
    }
}

pub fn get_process_name(pid: u32) -> Option<String> {
    get_process_info(pid).map(|(_, name)| name)
}

pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SAFETY: OpenProcess requests PROCESS_QUERY_LIMITED_INFORMATION, which is the least-privileged
    // query right. If opened, GetExitCodeProcess checks status and CloseHandle closes the handle cleanly.
    // If OpenProcess fails with ERROR_ACCESS_DENIED, the process is alive but kernel anti-cheat or
    // system security has protected its process handle.
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let mut exit_code = 0u32;
                let res = GetExitCodeProcess(handle, &mut exit_code);
                let _ = CloseHandle(handle);
                if res.is_ok() {
                    return exit_code == 259; // STILL_ACTIVE
                }
                false
            }
            Err(_) => {
                windows::Win32::Foundation::GetLastError()
                    == windows::Win32::Foundation::ERROR_ACCESS_DENIED
            }
        }
    }
}

pub struct SessionTracker {
    elapsed: Duration,
    was_playing: bool,
    inactive_duration: Duration,
    game_pid: Option<u32>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            was_playing: false,
            inactive_duration: Duration::ZERO,
            game_pid: None,
        }
    }

    pub fn tick(&mut self, is_playing: bool, delta: Duration) {
        self.tick_with_pid(is_playing, None, delta);
    }

    pub fn tick_with_pid(&mut self, is_playing: bool, current_pid: Option<u32>, delta: Duration) {
        if is_playing {
            self.elapsed += delta;
            self.was_playing = true;
            self.inactive_duration = Duration::ZERO;
            if let Some(pid) = current_pid {
                self.game_pid = Some(pid);
            }
        } else if self.was_playing {
            self.inactive_duration += delta;

            // Condition A: If the tracked game process terminated, player definitely quit the game
            let process_exited = if let Some(pid) = self.game_pid {
                !is_process_alive(pid)
            } else {
                false
            };

            // Condition B: Inactivity grace period (5 minutes) exceeded
            let timed_out = self.inactive_duration >= Duration::from_secs(300);

            if process_exited || timed_out {
                self.reset();
            }
        }
    }

    pub fn elapsed_playtime(&self) -> Duration {
        self.elapsed
    }

    pub fn is_active(&self) -> bool {
        self.was_playing
    }

    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.was_playing = false;
        self.inactive_duration = Duration::ZERO;
        self.game_pid = None;
    }
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_session(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    format!("{:02}H {:02}M", hours, mins)
}

pub fn detect_game_fullscreen() -> (bool, Option<u32>) {
    detect_game_fullscreen_filtered(&[], &[])
}

pub fn detect_game_fullscreen_filtered(
    custom_ignored: &[String],
    custom_games: &[String],
) -> (bool, Option<u32>) {
    // SAFETY: Queries the system foreground window, PID, process name, shell notification state,
    // and monitor bounds. All Win32 structs (RECT, MONITORINFO) are properly zero-initialized with cbSize.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return (false, None);
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return (false, None);
        }

        // Filter out desktop/taskbar windows (Progman, WorkerW, Shell_TrayWnd)
        if let Some(class) = get_window_class_name(hwnd) {
            if is_desktop_or_shell_window(&class) {
                return (false, None);
            }
        }

        // Fallback: Shell32 D3D Exclusive Fullscreen notification state for games where window rect
        // might not reflect standard desktop monitor coordinates
        let is_d3d_exclusive = if let Ok(state) = SHQueryUserNotificationState() {
            state == QUNS_RUNNING_D3D_FULL_SCREEN
        } else {
            false
        };

        // Retrieve process executable path and filename
        let (full_path, proc_name) = match get_process_info(pid) {
            Some(info) => info,
            None => {
                // If kernel anti-cheat restricts process info querying but D3D exclusive fullscreen is active,
                // treat as valid game
                if is_d3d_exclusive {
                    return (true, Some(pid));
                }
                return (false, None);
            }
        };
        let full_path_norm = full_path.to_lowercase().replace('/', "\\");
        let proc_lower = proc_name.to_lowercase();
        let proc_clean = proc_lower.strip_suffix(".exe").unwrap_or(&proc_lower);

        // User explicit ignore list takes precedence
        if matches_process_filter(proc_clean, &full_path_norm, custom_ignored) {
            return (false, None);
        }

        // Check foreground window bounds against display resolution (multi-monitor aware)
        let mut is_fullscreen = false;
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hmon, &mut mi).as_bool() {
                let m = mi.rcMonitor;
                if rect.left <= m.left
                    && rect.top <= m.top
                    && rect.right >= m.right
                    && rect.bottom >= m.bottom
                {
                    is_fullscreen = true;
                }
            } else {
                let screen_w = GetSystemMetrics(SM_CXSCREEN);
                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                if rect.left <= 0
                    && rect.top <= 0
                    && rect.right >= screen_w
                    && rect.bottom >= screen_h
                {
                    is_fullscreen = true;
                }
            }
        }

        if !is_fullscreen && !is_d3d_exclusive {
            return (false, None);
        }

        // Positive Game Identification:
        // 1. Explicit user custom game override (matches name, name without .exe, or path substring)
        // 2. Standard game library install directory (Steam, Epic, Xbox, GOG, Riot, Ubisoft, EA, Battle.net, \Games\)
        // 3. Exclusive Direct3D mode
        let is_custom_game = matches_process_filter(proc_clean, &full_path_norm, custom_games);

        let is_game = is_custom_game || is_game_path(&full_path_norm) || is_d3d_exclusive;

        if is_game {
            (true, Some(pid))
        } else {
            (false, None)
        }
    }
}

/// Matches a process name and normalized full path against user filter entries (e.g. custom_ignored or custom_games).
/// Strips `.exe` case-insensitively, normalizes path slashes, and ignores empty entries.
pub fn matches_process_filter(proc_clean: &str, full_path_norm: &str, filters: &[String]) -> bool {
    filters.iter().any(|f| {
        let trimmed = f.trim();
        if trimmed.is_empty() {
            return false;
        }
        let trimmed_lower = trimmed.to_lowercase().replace('/', "\\");
        let clean = trimmed_lower.strip_suffix(".exe").unwrap_or(&trimmed_lower);
        clean == proc_clean
            || (trimmed_lower.contains('\\') && full_path_norm.contains(&trimmed_lower))
    })
}

pub fn is_game_fullscreen() -> bool {
    detect_game_fullscreen().0
}
