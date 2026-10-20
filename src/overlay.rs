use crate::renderer::{render_split_pod_card, PixelBuffer};
use std::sync::Once;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromWindow, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
    GetSystemMetrics, KillTimer, PeekMessageW, RegisterClassW, SetTimer, SetWindowPos, ShowWindow,
    UpdateLayeredWindow, HCURSOR, HICON, HWND_TOPMOST, MSG, PM_REMOVE, SM_CXSCREEN, SWP_NOACTIVATE,
    SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

static REGISTER_OVERLAY_CLASS: Once = Once::new();
const TIMER_ANIM_ID: usize = 101;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationState {
    Hidden,
    SlideIn {
        start_time: Instant,
        duration: Duration,
    },
    Hold {
        until: Instant,
    },
    SlideOut {
        start_time: Instant,
        duration: Duration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AnimationPhase {
    In,
    Out,
}

fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

pub struct OverlayWindow {
    hwnd: HWND,
    width: i32,
    height: i32,
    anim_state: AnimationState,
    hold_duration: Duration,
    position: String,
    target_pos: (i32, i32),
    start_y: i32,
    cached_buf: Option<PixelBuffer>,
}

unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

impl OverlayWindow {
    pub fn new(position: &str, duration_secs: u32) -> windows::core::Result<Self> {
        let class_name = w!("TimeReminderOverlayClass");
        let width = 270;
        let height = 66;

        // SAFETY: Window class registration and window creation are performed
        // on the main UI thread with valid class names and styles.
        unsafe {
            REGISTER_OVERLAY_CLASS.call_once(|| {
                let wc = WNDCLASSW {
                    style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
                    lpfnWndProc: Some(overlay_wndproc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                        .unwrap_or_default()
                        .into(),
                    hIcon: HICON::default(),
                    hCursor: HCURSOR::default(),
                    hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                    lpszMenuName: windows::core::PCWSTR::null(),
                    lpszClassName: class_name,
                };

                let _ = RegisterClassW(&wc);
            });

            let hinstance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?;
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_TRANSPARENT
                    | WS_EX_NOACTIVATE
                    | WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW,
                class_name,
                w!("Time Reminder Overlay"),
                WS_POPUP,
                0,
                0,
                width,
                height,
                None,
                None,
                hinstance,
                None,
            )?;

            Ok(Self {
                hwnd,
                width,
                height,
                anim_state: AnimationState::Hidden,
                hold_duration: Duration::from_secs(duration_secs as u64),
                position: position.to_string(),
                target_pos: (0, 0),
                start_y: 0,
                cached_buf: None,
            })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn anim_state(&self) -> AnimationState {
        self.anim_state
    }

    pub fn is_animating(&self) -> bool {
        self.anim_state != AnimationState::Hidden
    }

    pub fn calculate_positions(&self) -> (i32, i32, i32) {
        // SAFETY: Querying foreground window metrics and monitor bounds using Win32 API.
        // MONITORINFO is properly initialized with its cbSize before querying.
        unsafe {
            let hwnd_fg = GetForegroundWindow();
            let hmon = MonitorFromWindow(hwnd_fg, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let (screen_x, screen_y, screen_w) = if GetMonitorInfoW(hmon, &mut mi).as_bool() {
                (
                    mi.rcMonitor.left,
                    mi.rcMonitor.top,
                    mi.rcMonitor.right - mi.rcMonitor.left,
                )
            } else {
                (0, 0, GetSystemMetrics(SM_CXSCREEN))
            };

            let (target_x, target_y) = match self.position.as_str() {
                "top-left" => (screen_x + 24, screen_y + 24),
                "top-center" => (screen_x + (screen_w - self.width) / 2, screen_y + 20),
                _ => (screen_x + screen_w - self.width - 24, screen_y + 24), // default top-right
            };

            // Start offscreen above monitor
            let start_y = screen_y - self.height - 12;

            (target_x, target_y, start_y)
        }
    }

    pub fn trigger(&mut self, time_str: &str, ampm_str: &str, session_str: &str) {
        let buf = render_split_pod_card(time_str, ampm_str, session_str, true);
        self.cached_buf = Some(buf);

        let (target_x, target_y, start_y) = self.calculate_positions();
        self.target_pos = (target_x, target_y);
        self.start_y = start_y;

        self.anim_state = AnimationState::SlideIn {
            start_time: Instant::now(),
            duration: Duration::from_millis(300),
        };

        // Position initially offscreen with alpha 0
        // SAFETY: self.hwnd is a valid top-level window. SetWindowPos, ShowWindow, and SetTimer
        // configure window positioning, visibility, and the 16ms animation timer.
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                target_x,
                start_y,
                self.width,
                self.height,
                SWP_NOACTIVATE,
            );
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = SetTimer(self.hwnd, TIMER_ANIM_ID, 16, None);
        }

        self.apply_frame(0.0, AnimationPhase::In);
    }

    pub fn reposition(&mut self) {
        let (target_x, target_y, start_y) = self.calculate_positions();
        self.target_pos = (target_x, target_y);
        self.start_y = start_y;
    }

    pub fn update(&mut self) -> bool {
        // Pump local window messages
        // SAFETY: PeekMessageW and DispatchMessageW safely drain thread message queue for self.hwnd.
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, self.hwnd, 0, 0, PM_REMOVE).as_bool() {
                let _ = DispatchMessageW(&msg);
            }
        }

        let now = Instant::now();
        match self.anim_state {
            AnimationState::Hidden => false,
            AnimationState::SlideIn {
                start_time,
                duration,
            } => {
                let elapsed = now.saturating_duration_since(start_time);
                if elapsed >= duration {
                    self.anim_state = AnimationState::Hold {
                        until: now + self.hold_duration,
                    };
                    self.apply_frame(1.0, AnimationPhase::In);
                    // CPU Optimization: Window reached final resting position.
                    // Suspend high-frequency 16ms timer and schedule wake-up when hold elapses.
                    // SAFETY: self.hwnd is a valid window handle.
                    unsafe {
                        let _ = KillTimer(self.hwnd, TIMER_ANIM_ID);
                        let hold_ms = self.hold_duration.as_millis().max(10) as u32;
                        let _ = SetTimer(self.hwnd, TIMER_ANIM_ID, hold_ms, None);
                    }
                } else {
                    let p = elapsed.as_secs_f32() / duration.as_secs_f32();
                    self.apply_frame(p, AnimationPhase::In);
                }
                true
            }
            AnimationState::Hold { until } => {
                if now >= until {
                    self.anim_state = AnimationState::SlideOut {
                        start_time: now,
                        duration: Duration::from_millis(250),
                    };
                    // Resume 16ms timer for smooth slide-out animation
                    // SAFETY: self.hwnd is a valid window handle.
                    unsafe {
                        let _ = SetTimer(self.hwnd, TIMER_ANIM_ID, 16, None);
                    }
                    self.apply_frame(0.0, AnimationPhase::Out);
                }
                // During stationary hold, skip GDI rendering entirely to conserve 100% CPU
                true
            }
            AnimationState::SlideOut {
                start_time,
                duration,
            } => {
                let elapsed = now.saturating_duration_since(start_time);
                if elapsed >= duration {
                    self.anim_state = AnimationState::Hidden;
                    // SAFETY: self.hwnd is a valid window handle.
                    unsafe {
                        let _ = KillTimer(self.hwnd, TIMER_ANIM_ID);
                        let _ = ShowWindow(self.hwnd, SW_HIDE);
                    }
                    false
                } else {
                    let p = elapsed.as_secs_f32() / duration.as_secs_f32();
                    self.apply_frame(p, AnimationPhase::Out);
                    true
                }
            }
        }
    }

    fn apply_frame(&mut self, progress: f32, phase: AnimationPhase) {
        let p = progress.clamp(0.0, 1.0);
        let (y, alpha) = match phase {
            AnimationPhase::In => {
                let t = ease_out_cubic(p);
                let cur_y = (self.start_y as f32 + (self.target_pos.1 - self.start_y) as f32 * t)
                    .round() as i32;
                let cur_alpha = (p * 255.0).round().clamp(0.0, 255.0) as u8;
                (cur_y, cur_alpha)
            }
            AnimationPhase::Out => {
                let t = ease_in_cubic(p);
                let cur_y = (self.target_pos.1 as f32
                    + (self.start_y - self.target_pos.1) as f32 * t)
                    .round() as i32;
                let cur_alpha = ((1.0 - p) * 255.0).round().clamp(0.0, 255.0) as u8;
                (cur_y, cur_alpha)
            }
        };

        if let Some(ref buf) = self.cached_buf {
            self.update_surface(buf, alpha);
        }

        // SAFETY: self.hwnd is valid; SetWindowPos repositions overlay on screen without taking input focus.
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                self.target_pos.0,
                y,
                self.width,
                self.height,
                SWP_NOACTIVATE,
            );
        }
    }

    pub fn update_surface(&self, buf: &PixelBuffer, alpha: u8) {
        // SAFETY: All GDI objects (screen_dc, mem_dc, bitmap, old_bmp) are created, selected,
        // and safely deleted or released before returning. Memory copy respects exact buffer bounds.
        unsafe {
            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return;
            }

            let mem_dc = match CreateCompatibleDC(screen_dc) {
                dc if !dc.is_invalid() => dc,
                _ => {
                    ReleaseDC(None, screen_dc);
                    return;
                }
            };

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: buf.width as i32,
                    biHeight: -(buf.height as i32), // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default()],
            };

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(b) if !b.is_invalid() => b,
                _ => {
                    let _ = DeleteDC(mem_dc);
                    ReleaseDC(None, screen_dc);
                    return;
                }
            };

            if !bits.is_null() {
                // SAFETY: bits points to memory allocated by CreateDIBSection with size buf.width * buf.height * 4.
                // buf.data is a contiguous slice of exactly that size.
                std::ptr::copy_nonoverlapping(
                    buf.data.as_ptr() as *const u8,
                    bits as *mut u8,
                    buf.data.len() * 4,
                );
            }

            let old_bmp = SelectObject(mem_dc, bitmap);

            let pt_src = POINT { x: 0, y: 0 };
            let sz = SIZE {
                cx: buf.width as i32,
                cy: buf.height as i32,
            };

            let blend = BLENDFUNCTION {
                BlendOp: 0, // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: alpha,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                None,
                Some(&sz),
                mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        // SAFETY: self.hwnd is owned by OverlayWindow and destroyed on drop.
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_ANIM_ID);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
