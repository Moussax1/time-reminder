use crate::config::Config;
use crate::renderer::PixelBuffer;
use crate::tray::log_debug;
use std::sync::{Mutex, Once};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromPoint, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetCursorPos, GetMessageW, GetSystemMetrics, LoadCursorW, PostMessageW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetCursor, SetForegroundWindow, SetWindowPos, ShowWindow,
    SystemParametersInfoW, TranslateMessage, UpdateLayeredWindow, HICON, HTCAPTION,
    HWND_TOPMOST, IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SWP_SHOWWINDOW, SW_SHOW,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ULW_ALPHA, WM_ACTIVATE, WM_CLOSE, WM_DESTROY,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_NCLBUTTONDOWN, WM_NULL, WM_QUIT, WM_SETCURSOR, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
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
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

pub const BTN_CLOSE_X: ButtonRect = ButtonRect::new(430, 10, 40, 25);
pub const BTN_BOOT_ON: ButtonRect = ButtonRect::new(35, 260, 200, 30);
pub const BTN_BOOT_OFF: ButtonRect = ButtonRect::new(245, 260, 200, 30);
pub const BTN_EXIT: ButtonRect = ButtonRect::new(25, 325, 105, 32);
pub const BTN_TEST: ButtonRect = ButtonRect::new(140, 325, 105, 32);
pub const BTN_CANCEL: ButtonRect = ButtonRect::new(255, 325, 85, 32);
pub const BTN_SAVE: ButtonRect = ButtonRect::new(350, 325, 105, 32);

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsAction {
    Save(Config),
    TestNotif(Config),
    ExitApp,
    Cancel,
}

pub struct SettingsState {
    pub config: Config,
    pub action: SettingsAction,
    pub was_active: bool,
    pub closed: bool,
}

static GLOBAL_SETTINGS_STATE: Mutex<Option<SettingsState>> = Mutex::new(None);
static REGISTER_SETTINGS_CLASS: Once = Once::new();

fn request_close(hwnd: HWND, action: SettingsAction) {
    log_debug(&format!("request_close: hwnd={:?}, action={:?}", hwnd, action));
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
        unsafe {
            let _ = DestroyWindow(hwnd);
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

            if let Ok(mut guard) = GLOBAL_SETTINGS_STATE.lock() {
                if let Some(state) = guard.as_mut() {
                    let mut needs_redraw = false;

                    // Intervals: 15m, 30m, 45m, 60m
                    let intervals = [(35, 15), (140, 30), (245, 45), (350, 60)];
                    for (bx, val) in intervals {
                        if x >= bx && x < bx + 95 && y >= 65 && y < 65 + 30 {
                            state.config.interval_minutes = val;
                            needs_redraw = true;
                        }
                    }

                    // Positions: Top-Right (35), Top-Center (175), Top-Left (315)
                    let positions = [
                        (35, "top-right"),
                        (175, "top-center"),
                        (315, "top-left"),
                    ];
                    for (bx, pos_str) in positions {
                        if x >= bx && x < bx + 130 && y >= 130 && y < 130 + 30 {
                            state.config.position = pos_str.to_string();
                            needs_redraw = true;
                        }
                    }

                    // Formats: 12-Hour (35, w=200), 24-Hour (245, w=200)
                    if x >= 35 && x < 235 && y >= 195 && y < 195 + 30 {
                        state.config.time_format = "12h".to_string();
                        needs_redraw = true;
                    } else if x >= 245 && x < 445 && y >= 195 && y < 195 + 30 {
                        state.config.time_format = "24h".to_string();
                        needs_redraw = true;
                    }

                    // Start on Boot: Enabled (35, w=200), Disabled (245, w=200)
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
            if key == 0x1B /* VK_ESCAPE */ {
                request_close(hwnd, SettingsAction::Cancel);
                return LRESULT(0);
            } else if key == 0x0D /* VK_RETURN */ {
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
            log_debug(&format!("settings_wndproc: WM_ACTIVATE active={}, hwnd={:?}", active, hwnd));
            if active != 0 {
                // Window activated
                if let Ok(mut guard) = GLOBAL_SETTINGS_STATE.lock() {
                    if let Some(state) = guard.as_mut() {
                        state.was_active = true;
                    }
                }
            } else {
                // WA_INACTIVE: user clicked outside or switched window -> dismiss flyout
                let should_close = if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        log_debug(&format!("settings_wndproc: WA_INACTIVE was_active={}, closed={}", state.was_active, state.closed));
                        state.was_active && !state.closed
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
            unsafe {
                let _ = PostMessageW(HWND::default(), WM_NULL, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
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
    let height = 375;
    let mut buf = PixelBuffer::new(width, height, 0x00000000);

    let col_bg = 0xFF000000;
    let col_white = 0xFFFFFFFF;
    let col_btn_bg = 0xFF141414;
    let col_btn_border = 0xFF444444;
    let col_muted = 0xFF888888;
    let col_black = 0xFF000000;
    let col_exit_bg = 0xFF280A0A;
    let col_exit_red = 0xFFFF4444;

    // Outer Dialog (r=16, 3px white border)
    buf.draw_rounded_rect(2, 2, width as i32 - 4, height as i32 - 4, 16, col_bg, col_white, 3);

    // Titlebar
    buf.draw_text("► TIME REMINDER CONFIG", 25, 20, 1, col_white);
    buf.draw_text("[X]", 438, 20, 1, col_muted);

    // Horizontal Line
    for px in 20..460 {
        buf.set_pixel(px, 38, col_white);
    }

    // 1. INTERVAL SECTION
    buf.draw_text("INTERVAL:", 25, 50, 1, col_muted);
    let intervals = [(35, 15, "15M"), (140, 30, "30M"), (245, 45, "45M"), (350, 60, "60M")];
    for (bx, val, label) in intervals {
        let is_selected = cfg.interval_minutes == val;
        let (bg, border, text_col) = if is_selected {
            (col_white, col_white, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 65, 95, 30, 6, bg, border, 1);
        let text_with_mark = if is_selected {
            format!("{} ✓", label)
        } else {
            label.to_string()
        };
        let text_w = text_with_mark.len() as i32 * 9 - 1;
        let text_x = bx + (95 - text_w) / 2;
        buf.draw_text(&text_with_mark, text_x, 74, 1, text_col);
    }

    // 2. POSITION SECTION
    buf.draw_text("SCREEN POSITION:", 25, 115, 1, col_muted);
    let positions = [
        (35, "top-right", "TOP-RIGHT"),
        (175, "top-center", "TOP-CENTER"),
        (315, "top-left", "TOP-LEFT"),
    ];
    for (bx, pos_val, label) in positions {
        let is_selected = cfg.position == pos_val;
        let (bg, border, text_col) = if is_selected {
            (col_white, col_white, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 130, 130, 30, 6, bg, border, 1);
        let text_w = label.len() as i32 * 9 - 1;
        let text_x = bx + (130 - text_w) / 2;
        buf.draw_text(label, text_x, 139, 1, text_col);
    }

    // 3. CLOCK FORMAT SECTION
    buf.draw_text("CLOCK FORMAT:", 25, 180, 1, col_muted);
    let formats = [
        (35, "12h", "12-HOUR (AM/PM)", 200),
        (245, "24h", "24-HOUR (MILITARY)", 200),
    ];
    for (bx, fmt_val, label, bw) in formats {
        let is_selected = cfg.time_format == fmt_val;
        let (bg, border, text_col) = if is_selected {
            (col_white, col_white, col_black)
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
    let boot_options = [
        (35, true, "ENABLED", 200),
        (245, false, "DISABLED", 200),
    ];
    for (bx, val, label, bw) in boot_options {
        let is_selected = cfg.run_on_startup == val;
        let (bg, border, text_col) = if is_selected {
            (col_white, col_white, col_black)
        } else {
            (col_btn_bg, col_btn_border, col_muted)
        };
        buf.draw_rounded_rect(bx, 260, bw, 30, 6, bg, border, 1);
        let text_with_mark = if is_selected {
            format!("{} ✓", label)
        } else {
            label.to_string()
        };
        let text_w = text_with_mark.len() as i32 * 9 - 1;
        let text_x = bx + (bw - text_w) / 2;
        buf.draw_text(&text_with_mark, text_x, 269, 1, text_col);
    }

    // Divider
    for px in 20..460 {
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

    // 3. CANCEL (dark bg, muted text)
    buf.draw_rounded_rect(255, 325, 85, 32, 6, col_btn_bg, col_btn_border, 1);
    let cancel_w = "CANCEL".len() as i32 * 9 - 1;
    buf.draw_text("CANCEL", 255 + (85 - cancel_w) / 2, 337, 1, col_muted);

    // 4. SAVE (white solid bg, black text)
    buf.draw_rounded_rect(350, 325, 105, 32, 6, col_white, col_white, 1);
    let save_w = "SAVE".len() as i32 * 9 - 1;
    buf.draw_text("SAVE", 350 + (105 - save_w) / 2, 337, 1, col_black);

    buf
}

fn render_and_update(hwnd: HWND, cfg: &Config) {
    let buf = render_settings_buffer(cfg);
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
    let _ = unsafe { GetCursorPos(&mut cursor_pt) };

    let mut work_area = windows::Win32::Foundation::RECT::default();
    let mut found = false;

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
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        work_area.right = screen_w;
        work_area.bottom = screen_h - 48; // Assume standard taskbar height fallback
    }

    calculate_tray_flyout_pos(width, height, work_area.right, work_area.bottom)
}

pub fn enable_dpi_awareness() {
    unsafe {
        // SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4)
        if let Ok(user32) = windows::Win32::System::LibraryLoader::GetModuleHandleW(w!("user32.dll")) {
            let proc_name = std::ffi::CString::new("SetProcessDpiAwarenessContext").unwrap();
            if let Some(proc) = windows::Win32::System::LibraryLoader::GetProcAddress(
                user32,
                windows::core::PCSTR(proc_name.as_ptr() as *const u8),
            ) {
                type SetProcessDpiAwarenessContextFn =
                    unsafe extern "system" fn(isize) -> windows::Win32::Foundation::BOOL;
                let func: SetProcessDpiAwarenessContextFn = std::mem::transmute(proc);
                let _ = func(-4);
            }
        }
    }
}

pub fn show_settings_dialog(current_cfg: &Config) -> SettingsAction {
    enable_dpi_awareness();
    let class_name = w!("TimeReminderSettingsClass");
    let state = SettingsState {
        config: current_cfg.clone(),
        action: SettingsAction::Cancel,
        was_active: false,
        closed: false,
    };

    if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
        *g = Some(state);
    }

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
        let height = 375;
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

        render_and_update(hwnd, current_cfg);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            width,
            height,
            SWP_SHOWWINDOW,
        );
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetCursor(LoadCursorW(None, IDC_ARROW).unwrap_or_default());

        if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
            if let Some(s) = g.as_mut() {
                s.was_active = true;
            }
        }

        // Run modal message loop strictly scoped to hwnd so main thread timer/hotkey messages remain queued
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

            let res = GetMessageW(&mut msg, hwnd, 0, 0);
            if res.0 <= 0 || msg.message == WM_QUIT {
                log_debug(&format!(
                    "show_settings_dialog: received WM_QUIT or exit, wParam={}",
                    msg.wParam.0
                ));
                PostQuitMessage(msg.wParam.0 as i32);
                break;
            }

            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }

    if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
        g.take().map(|s| s.action).unwrap_or(SettingsAction::Cancel)
    } else {
        SettingsAction::Cancel
    }
}

