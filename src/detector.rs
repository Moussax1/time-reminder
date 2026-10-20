use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId, SM_CXSCREEN,
    SM_CYSCREEN,
};

pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut exit_code = 0u32;
            let res = GetExitCodeProcess(handle, &mut exit_code);
            let _ = CloseHandle(handle);
            if res.is_ok() {
                return exit_code == 259; // STILL_ACTIVE
            }
        }
        false
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

pub fn format_session(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    format!("{:02}H {:02}M", hours, mins)
}

pub fn detect_game_fullscreen() -> (bool, Option<u32>) {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut pid = 0u32;
        if !hwnd.is_invalid() {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        let opt_pid = if pid > 0 { Some(pid) } else { None };

        // Method 1: Shell32 D3D Fullscreen notification state
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_BUSY {
                return (true, opt_pid);
            }
        }

        // Method 2: Check foreground window bounds against display resolution (multi-monitor aware)
        if !hwnd.is_invalid() {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(hmon, &mut mi).as_bool() {
                    let m = mi.rcMonitor;
                    if rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom {
                        return (true, opt_pid);
                    }
                } else {
                    let screen_w = GetSystemMetrics(SM_CXSCREEN);
                    let screen_h = GetSystemMetrics(SM_CYSCREEN);
                    if rect.left <= 0 && rect.top <= 0 && rect.right >= screen_w && rect.bottom >= screen_h {
                        return (true, opt_pid);
                    }
                }
            }
        }
        (false, None)
    }
}

pub fn is_game_fullscreen() -> bool {
    detect_game_fullscreen().0
}

