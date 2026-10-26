use crate::config::Config;
use crate::renderer::PixelBuffer;
use crate::tray::log_debug;
use std::sync::{Mutex, Once};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateRoundRectRgn, DeleteDC, DeleteObject, GetDC,
    GetMonitorInfoW, MonitorFromPoint, ReleaseDC, SelectObject, SetWindowRgn, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetCursorPos, GetMessageW, GetSystemMetrics, LoadCursorW, PostMessageW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetCursor, SetForegroundWindow, SetWindowPos, ShowWindow,
    SystemParametersInfoW, TranslateMessage, UpdateLayeredWindow, HICON, HTCAPTION, HWND_TOPMOST,
    IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SWP_SHOWWINDOW, SW_SHOW,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ULW_ALPHA, WM_ACTIVATE, WM_CLOSE, WM_DESTROY, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_NCLBUTTONDOWN, WM_NULL, WM_QUIT, WM_SETCURSOR, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

// Rect for button hit-testing
#[derive(Clone, Copy)]
pub struct ButtonRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl ButtonRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        (self.x..self.x + self.w).contains(&px) && (self.y..self.y + self.h).contains(&py)
    }
}

pub const BTN_CLOSE_X: ButtonRect = ButtonRect::new(425, 8, 35, 28);
pub const BTN_BOOT_ON: ButtonRect = ButtonRect::new(25, 260, 210, 30);
pub const BTN_BOOT_OFF: ButtonRect = ButtonRect::new(245, 260, 210, 30);
pub const BTN_EXIT: ButtonRect = ButtonRect::new(25, 325, 105, 32);
pub const BTN_TEST: ButtonRect = ButtonRect::new(140, 325, 105, 32);
pub const BTN_CANCEL: ButtonRect = ButtonRect::new(255, 325, 85, 32);
pub const BTN_SAVE: ButtonRect = ButtonRect::new(350, 325, 105, 32);
pub const BTN_RESET_SESSION: ButtonRect = ButtonRect::new(315, 360, 140, 24);

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsAction {
    Save(Config),
    TestNotif(Config),
    ResetSession(Config),
    ExitApp,
    Cancel,
}

pub struct SettingsState {
    pub config: Config,
    pub action: SettingsAction,
    pub closed: bool,
    pub opened_at: std::time::Instant,
}

static GLOBAL_SETTINGS_STATE: Mutex<Option<SettingsState>> = Mutex::new(None);
static REGISTER_SETTINGS_CLASS: Once = Once::new();

fn request_close(hwnd: HWND, action: SettingsAction) {
    log_debug(&format!(
        "request_close: hwnd={:?}, action={:?}",
        hwnd, action
    ));
    let mut already_closed = false;
    if let Ok(mut guard) = GLOBAL_SETTINGS_STATE.lock() {
        if let Some(state) = guard.as_mut() {
            if state.closed {
                already_closed = true;
            } else {
                state.action = action;
                state.closed = true;
            }
        }
    }
    if !already_closed {
        // SAFETY: hwnd is the valid settings dialog window handle. DestroyWindow cleans up
        // the Win32 window resources and PostMessageW unblocks any waiting message loops.
        unsafe {
            if !hwnd.is_invalid() {
                let _ = DestroyWindow(hwnd);
            }
            let _ = PostMessageW(HWND::default(), WM_NULL, WPARAM(0), LPARAM(0));
        }
    }
}

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i32;

            // Titlebar drag support (except close button at x: 425..480)
            if y <= 40 && x < 425 {
                // SAFETY: ReleaseCapture and SendMessageW with WM_NCLBUTTONDOWN and HTCAPTION
                // delegate standard window dragging to the Windows shell without capturing cursor.
                let _ = ReleaseCapture();
                let _ = SendMessageW(
                    hwnd,
                    WM_NCLBUTTONDOWN,
                    WPARAM(HTCAPTION as usize),
                    LPARAM(0),
                );
                return LRESULT(0);
            }

            // Close [X]
            if BTN_CLOSE_X.contains(x, y) {
                request_close(hwnd, SettingsAction::Cancel);
                return LRESULT(0);
            }

            // Bottom Bar Buttons
            if BTN_EXIT.contains(x, y) {
                request_close(hwnd, SettingsAction::ExitApp);
                return LRESULT(0);
            }
            if BTN_TEST.contains(x, y) {
                let cfg = if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                    guard.as_ref().map(|s| s.config.clone())
                } else {
                    None
                };
                if let Some(cfg) = cfg {
                    request_close(hwnd, SettingsAction::TestNotif(cfg));
                }
                return LRESULT(0);
            }
            if BTN_CANCEL.contains(x, y) {
                request_close(hwnd, SettingsAction::Cancel);
                return LRESULT(0);
            }
            if BTN_SAVE.contains(x, y) {
                let cfg = if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                    guard.as_ref().map(|s| s.config.clone())
                } else {
                    None
                };
                if let Some(cfg) = cfg {
                    request_close(hwnd, SettingsAction::Save(cfg));
                }
                return LRESULT(0);
            }
            if BTN_RESET_SESSION.contains(x, y) {
                let cfg = if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                    guard.as_ref().map(|s| s.config.clone())
                } else {
                    None
                };
                if let Some(cfg) = cfg {
                    request_close(hwnd, SettingsAction::ResetSession(cfg));
                }
                return LRESULT(0);
            }

            if let Ok(mut guard) = GLOBAL_SETTINGS_STATE.lock() {
                if let Some(state) = guard.as_mut() {
                    let mut needs_redraw = false;

                    // Intervals: 15m, 30m, 45m, 60m (flush grid 25..455)
                    let intervals = [(25, 15), (135, 30), (245, 45), (355, 60)];
                    for (bx, val) in intervals {
                        if (bx..bx + 100).contains(&x) && (65..65 + 30).contains(&y) {
                            state.config.interval_minutes = val;
                            needs_redraw = true;
                        }
                    }

                    // Positions: Top-Left (25), Top-Center (172), Top-Right (319) (flush grid 25..455)
                    let positions = [(25, "top-left"), (172, "top-center"), (319, "top-right")];
                    for (bx, pos_str) in positions {
                        if (bx..bx + 136).contains(&x) && (130..130 + 30).contains(&y) {
                            state.config.position = pos_str.to_string();
                            needs_redraw = true;
                        }
                    }

                    // Formats: 12-Hour (25, w=210), 24-Hour (245, w=210) (flush grid 25..455)
                    if (25..235).contains(&x) && (195..195 + 30).contains(&y) {
                        state.config.time_format = "12h".to_string();
                        needs_redraw = true;
                    } else if (245..455).contains(&x) && (195..195 + 30).contains(&y) {
                        state.config.time_format = "24h".to_string();
                        needs_redraw = true;
                    }

                    // Start on Boot: Enabled (25, w=210), Disabled (245, w=210) (flush grid 25..455)
                    if BTN_BOOT_ON.contains(x, y) {
                        state.config.run_on_startup = true;
                        needs_redraw = true;
                    } else if BTN_BOOT_OFF.contains(x, y) {
                        state.config.run_on_startup = false;
                        needs_redraw = true;
                    }

                    if needs_redraw {
                        render_and_update(hwnd, &state.config);
                    }
                }
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let key = wparam.0 as u32;
            if key == 0x1B
            /* VK_ESCAPE */
            {
                request_close(hwnd, SettingsAction::Cancel);
                return LRESULT(0);
            } else if key == 0x0D
            /* VK_RETURN */
            {
                let cfg = if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                    guard.as_ref().map(|s| s.config.clone())
                } else {
                    None
                };
                if let Some(cfg) = cfg {
                    request_close(hwnd, SettingsAction::Save(cfg));
                    return LRESULT(0);
                }
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let active = (wparam.0 & 0xFFFF) as u32;
            log_debug(&format!(
                "settings_wndproc: WM_ACTIVATE active={}, hwnd={:?}",
                active, hwnd
            ));
            if active == 0 {
                // WA_INACTIVE: user clicked outside or switched window -> dismiss flyout.
                // Guard: Only dismiss if open for >=250ms to avoid premature close during initial window creation.
                let should_close = if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        let open_time = state.opened_at.elapsed();
                        log_debug(&format!(
                            "settings_wndproc: WA_INACTIVE closed={}, elapsed={:?}",
                            state.closed, open_time
                        ));
                        !state.closed && open_time > std::time::Duration::from_millis(250)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_close {
                    request_close(hwnd, SettingsAction::Cancel);
                    return LRESULT(0);
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            request_close(hwnd, SettingsAction::Cancel);
            LRESULT(0)
        }
        WM_DESTROY => {
            log_debug("settings_wndproc: WM_DESTROY");
            if let Ok(mut guard) = GLOBAL_SETTINGS_STATE.lock() {
                if let Some(state) = guard.as_mut() {
                    state.closed = true;
                }
            }
            // SAFETY: Posts a null message to the calling thread's message queue to awaken
            // GetMessageW in the modal loop, allowing it to inspect state.closed and exit cleanly.
            unsafe {
                let _ = PostMessageW(HWND::default(), WM_NULL, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND => LRESULT(1),
        WM_SETCURSOR => {
            // SAFETY: Loads and applies the standard system arrow cursor (IDC_ARROW) over the window client area.
            unsafe {
                let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
                SetCursor(cursor);
            }
            LRESULT(1)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn render_settings_buffer(cfg: &Config) -> PixelBuffer {
    let width = 480;
    let height = 398;
    let mut buf = PixelBuffer::new(width, height, 0x00000000);

    let col_bg = 0xFF000000;
    let col_white = 0xFFFFFFFF;
    let col_btn_bg = 0xFF141414;
    let col_btn_border = 0xFF444444;
    let col_muted = 0xFFB0B0B0;
    let col_black = 0xFF000000;
    let col_exit_bg = 0xFF280A0A;
    let col_exit_red = 0xFFFF4444;
    let col_accent = 0xFF00FF9D;

    // Outer Dialog (r=16, 3px Cyber Mint brand border)
    buf.draw_rounded_rect(
        2,
        2,
        width as i32 - 4,
        height as i32 - 4,
        16,
        col_bg,
        col_accent,
        3,
    );

    // Titlebar
    buf.draw_text("■", 25, 20, 1, col_accent);
    buf.draw_text(" TIME REMINDER CONFIG", 34, 20, 1, col_white);
    buf.draw_text("[X]", 430, 20, 1, col_muted);

    // Horizontal Line (flush grid 25..455)
    for px in 25..456 {
        buf.set_pixel(px, 38, col_white);
    }

    // 1. INTERVAL SECTION
    buf.draw_text("INTERVAL:", 25, 50, 1, col_muted);
    let intervals = [
        (25, 15, "15M"),
        (135, 30, "30M"),
        (245, 45, "45M"),
        (355, 60, "60M"),
    ];
    for (bx, val, label) in intervals {
        let is_selected = cfg.interval_minutes == val;
        let (bg, border, text_col) = if is_selected {
            (col_accent, col_accent, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 65, 100, 30, 6, bg, border, 1);
        let text_w = label.len() as i32 * 9 - 1;
        let text_x = bx + (100 - text_w) / 2;
        buf.draw_text(label, text_x, 74, 1, text_col);
    }

    // 2. POSITION SECTION
    buf.draw_text("SCREEN POSITION:", 25, 115, 1, col_muted);
    let positions = [
        (25, "top-left", "TOP-LEFT"),
        (172, "top-center", "TOP-CENTER"),
        (319, "top-right", "TOP-RIGHT"),
    ];
    for (bx, pos_val, label) in positions {
        let is_selected = cfg.position == pos_val;
        let (bg, border, text_col) = if is_selected {
            (col_accent, col_accent, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 130, 136, 30, 6, bg, border, 1);
        let text_w = label.len() as i32 * 9 - 1;
        let text_x = bx + (136 - text_w) / 2;
        buf.draw_text(label, text_x, 139, 1, text_col);
    }

    // 3. CLOCK FORMAT SECTION
    buf.draw_text("CLOCK FORMAT:", 25, 180, 1, col_muted);
    let formats = [
        (25, "12h", "12-HOUR (AM/PM)", 210),
        (245, "24h", "24-HOUR (MILITARY)", 210),
    ];
    for (bx, fmt_val, label, bw) in formats {
        let is_selected = cfg.time_format == fmt_val;
        let (bg, border, text_col) = if is_selected {
            (col_accent, col_accent, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 195, bw, 30, 6, bg, border, 1);
        let text_w = label.len() as i32 * 9 - 1;
        let text_x = bx + (bw - text_w) / 2;
        buf.draw_text(label, text_x, 204, 1, text_col);
    }

    // 4. START ON BOOT SECTION
    buf.draw_text("START ON BOOT:", 25, 245, 1, col_muted);
    let boot_options = [(25, true, "ENABLED", 210), (245, false, "DISABLED", 210)];
    for (bx, val, label, bw) in boot_options {
        let is_selected = cfg.run_on_startup == val;
        let (bg, border, text_col) = if is_selected {
            (col_accent, col_accent, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 260, bw, 30, 6, bg, border, 1);
        let text_w = label.len() as i32 * 9 - 1;
        let text_x = bx + (bw - text_w) / 2;
        buf.draw_text(label, text_x, 269, 1, text_col);
    }

    // Divider (flush grid 25..455)
    for px in 25..456 {
        buf.set_pixel(px, 310, col_btn_border);
    }

    // 1. EXIT APP (dark red bg, bright red border & text)
    buf.draw_rounded_rect(25, 325, 105, 32, 6, col_exit_bg, col_exit_red, 1);
    let exit_w = "EXIT APP".len() as i32 * 9 - 1;
    buf.draw_text("EXIT APP", 25 + (105 - exit_w) / 2, 337, 1, col_exit_red);

    // 2. TEST OSD (dark bg, white text)
    buf.draw_rounded_rect(140, 325, 105, 32, 6, col_btn_bg, col_btn_border, 1);
    let test_w = "TEST OSD".len() as i32 * 9 - 1;
    buf.draw_text("TEST OSD", 140 + (105 - test_w) / 2, 337, 1, col_white);

    // 3. CANCEL (dark bg, white text)
    buf.draw_rounded_rect(255, 325, 85, 32, 6, col_btn_bg, col_btn_border, 1);
    let cancel_w = "CANCEL".len() as i32 * 9 - 1;
    buf.draw_text("CANCEL", 255 + (85 - cancel_w) / 2, 337, 1, col_white);

    // 4. SAVE (accent Cyber Mint pill, high contrast black text)
    buf.draw_rounded_rect(350, 325, 105, 32, 6, col_accent, col_accent, 1);
    let save_w = "SAVE".len() as i32 * 9 - 1;
    buf.draw_text("SAVE", 350 + (105 - save_w) / 2, 337, 1, col_black);

    // Footer Hints & Quick Actions (HOTKEY: ALT-T and [RESET SESSION])
    buf.draw_text("HOTKEY: ALT-T", 25, 368, 1, col_muted);
    buf.draw_text("[RESET SESSION]", 319, 368, 1, col_muted);

    buf
}

fn render_and_update(hwnd: HWND, cfg: &Config) {
    let buf = render_settings_buffer(cfg);
    // SAFETY: Allocates and paints a temporary 32-bit DIB section onto the layered window DC.
    // Screen DC and memory DC are properly released and deleted on all code paths.
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
                biHeight: -(buf.height as i32),
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
            // SAFETY: bits is allocated with size buf.width * buf.height * 4 by CreateDIBSection.
            // buf.data is a contiguous vector matching these dimensions.
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

        let blend = windows::Win32::Graphics::Gdi::BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
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

pub fn calculate_tray_flyout_pos(
    width: i32,
    height: i32,
    work_area_right: i32,
    work_area_bottom: i32,
) -> (i32, i32) {
    let margin_x = 12;
    let margin_y = 12;
    (
        work_area_right - width - margin_x,
        work_area_bottom - height - margin_y,
    )
}

pub fn get_tray_flyout_position(width: i32, height: i32) -> (i32, i32) {
    let mut cursor_pt = POINT::default();
    // SAFETY: GetCursorPos retrieves current mouse position into a zeroed POINT struct.
    let _ = unsafe { GetCursorPos(&mut cursor_pt) };

    let mut work_area = windows::Win32::Foundation::RECT::default();
    let mut found = false;

    // SAFETY: Querying nearest monitor and its work area bounds via Win32 API.
    unsafe {
        let hmon = MonitorFromPoint(cursor_pt, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            work_area = mi.rcWork;
            if work_area.right > work_area.left && work_area.bottom > work_area.top {
                found = true;
            }
        }
    }

    if !found {
        // SAFETY: Fallback to primary work area using SystemParametersInfoW with SPI_GETWORKAREA.
        unsafe {
            let _ = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut work_area as *mut _ as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
        }
    }

    if work_area.right <= work_area.left || work_area.bottom <= work_area.top {
        // SAFETY: Querying standard system metrics for primary monitor fallback.
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        work_area.right = screen_w;
        work_area.bottom = screen_h - 48; // Assume standard taskbar height fallback
    }

    calculate_tray_flyout_pos(width, height, work_area.right, work_area.bottom)
}

pub use crate::enable_dpi_awareness;

pub fn show_settings_dialog(current_cfg: &Config) -> SettingsAction {
    enable_dpi_awareness();
    let class_name = w!("TimeReminderSettingsClass");

    if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
        if let Some(state) = g.as_ref() {
            if !state.closed {
                log_debug("show_settings_dialog: dialog already open, ignoring duplicate call");
                return SettingsAction::Cancel;
            }
        }
        *g = Some(SettingsState {
            config: current_cfg.clone(),
            action: SettingsAction::Cancel,
            closed: false,
            opened_at: std::time::Instant::now(),
        });
    }

    // SAFETY: Initializes the modal settings GUI window, registers window class,
    // and pumps messages scoped to this window until dismissed.
    unsafe {
        let hinstance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
            Ok(h) => h,
            Err(_) => return SettingsAction::Cancel,
        };

        REGISTER_SETTINGS_CLASS.call_once(|| {
            let wc = WNDCLASSW {
                style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
                lpfnWndProc: Some(settings_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance.into(),
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                lpszMenuName: windows::core::PCWSTR::null(),
                lpszClassName: class_name,
            };

            let _ = RegisterClassW(&wc);
        });

        let width = 480;
        let height = 398;
        let (x, y) = get_tray_flyout_position(width, height);

        let hwnd = match CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!("Time Reminder Settings"),
            WS_POPUP,
            x,
            y,
            width,
            height,
            None,
            None,
            hinstance,
            None,
        ) {
            Ok(h) if !h.is_invalid() => h,
            _ => return SettingsAction::Cancel,
        };

        // Mask window region to clean rounded corners, eliminating any sharp corner artifacts
        let hrgn = CreateRoundRectRgn(0, 0, width + 1, height + 1, 33, 33);
        if !hrgn.is_invalid() && SetWindowRgn(hwnd, hrgn, true) == 0 {
            let _ = DeleteObject(hrgn);
        }

        render_and_update(hwnd, current_cfg);
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_SHOWWINDOW);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetCursor(LoadCursorW(None, IDC_ARROW).unwrap_or_default());

        // Run modal message loop with None so all thread messages (hotkey, timers, overlay)
        // are properly pumped and never starved or deadlocked.
        let mut msg = MSG::default();
        loop {
            let is_closed = GLOBAL_SETTINGS_STATE
                .lock()
                .map(|g| g.as_ref().map(|s| s.closed).unwrap_or(true))
                .unwrap_or(true);

            if is_closed {
                log_debug("show_settings_dialog: breaking modal loop because state.closed is true");
                break;
            }

            let res = GetMessageW(&mut msg, None, 0, 0);
            if res.0 <= 0 || msg.message == WM_QUIT {
                log_debug(&format!(
                    "show_settings_dialog: received WM_QUIT or exit, wParam={}",
                    msg.wParam.0
                ));
                if msg.message == WM_QUIT {
                    PostQuitMessage(msg.wParam.0 as i32);
                }
                break;
            }

            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }

    let action = if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
        g.take().map(|s| s.action).unwrap_or(SettingsAction::Cancel)
    } else {
        SettingsAction::Cancel
    };

    // Trim process working set after closing the configuration GUI to free all dialog allocations
    crate::trim_working_set();

    action
}
