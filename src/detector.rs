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

        // Method 2: Check foreground window bounds against display resolution
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
