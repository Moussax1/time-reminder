use crate::renderer::{render_split_pod_card, PixelBuffer};
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetSystemMetrics,
    PeekMessageW, RegisterClassW, SetWindowPos, ShowWindow, UpdateLayeredWindow,
    HCURSOR, HICON, PM_REMOVE, SM_CXSCREEN, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE,
    ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

pub struct OverlayWindow {
    hwnd: HWND,
    width: i32,
    height: i32,
    active_until: Option<Instant>,
    fade_start: Option<Instant>,
    duration: Duration,
    position: String,
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

        unsafe {
            let wc = WNDCLASSW {
                style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
                lpfnWndProc: Some(overlay_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.into(),
                hIcon: HICON::default(),
                hCursor: HCURSOR::default(),
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                lpszMenuName: windows::core::PCWSTR::null(),
                lpszClassName: class_name,
            };

            RegisterClassW(&wc);

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
                windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?,
                None,
            )?;

            Ok(Self {
                hwnd,
                width,
                height,
                active_until: None,
                fade_start: None,
                duration: Duration::from_secs(duration_secs as u64),
                position: position.to_string(),
            })
        }
    }

    pub fn trigger(&mut self, time_str: &str, ampm_str: &str, session_str: &str) {
        let buf = render_split_pod_card(time_str, ampm_str, session_str, true);
        self.update_surface(&buf, 255);
        self.reposition();

        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }

        let now = Instant::now();
        self.fade_start = Some(now);
        self.active_until = Some(now + self.duration);
    }

    pub fn reposition(&self) {
        unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let (x, y) = match self.position.as_str() {
                "top-left" => (24, 24),
                "top-center" => ((screen_w - self.width) / 2, 20),
                _ => (screen_w - self.width - 24, 24), // default top-right
            };

            let _ = SetWindowPos(
                self.hwnd,
                HWND::default(),
                x,
                y,
                self.width,
                self.height,
                SWP_NOACTIVATE,
            );
        }
    }

    pub fn update(&mut self) -> bool {
        // Pump local window messages
        unsafe {
            let mut msg = std::mem::zeroed();
            while PeekMessageW(&mut msg, self.hwnd, 0, 0, PM_REMOVE).as_bool() {
                let _ = DispatchMessageW(&msg);
            }
        }

        if let Some(active_until) = self.active_until {
            let now = Instant::now();
            if now > active_until {
                // Time expired -> hide
                unsafe {
                    let _ = ShowWindow(self.hwnd, SW_HIDE);
                }
                self.active_until = None;
                self.fade_start = None;
                return false;
            }
            return true;
        }
        false
    }

    pub fn update_surface(&self, buf: &PixelBuffer, alpha: u8) {
        unsafe {
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

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
            let bitmap = CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            ).unwrap();

            if !bits.is_null() {
                // Copy buffer bytes directly
                std::ptr::copy_nonoverlapping(
                    buf.data.as_ptr() as *const u8,
                    bits as *mut u8,
                    buf.data.len() * 4,
                );
            }

            let old_bmp = SelectObject(mem_dc, bitmap);

            let mut pt_src = POINT { x: 0, y: 0 };
            let mut sz = SIZE {
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
                Some(&mut sz),
                mem_dc,
                Some(&mut pt_src),
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
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
