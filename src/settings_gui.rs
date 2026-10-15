use crate::config::Config;
use crate::renderer::PixelBuffer;
use std::sync::{Arc, Mutex};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetSystemMetrics,
    PostQuitMessage, RegisterClassW, ShowWindow, TranslateMessage,
    UpdateLayeredWindow, HCURSOR, HICON, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW,
    ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED, WS_POPUP,
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
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

pub struct SettingsState {
    pub config: Config,
    pub changed: bool,
    pub closed: bool,
}

static GLOBAL_SETTINGS_STATE: Mutex<Option<Arc<Mutex<SettingsState>>>> = Mutex::new(None);

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i32;

            if let Ok(guard) = GLOBAL_SETTINGS_STATE.lock() {
                if let Some(state_arc) = guard.as_ref() {
                    if let Ok(mut state) = state_arc.lock() {
                    let mut needs_redraw = false;

                    // Close [X] (435, 15, 30, 20)
                    if x >= 430 && x <= 470 && y >= 10 && y <= 35 {
                        state.closed = true;
                        let _ = DestroyWindow(hwnd);
                        PostQuitMessage(0);
                        return LRESULT(0);
                    }

                    // Intervals: 15m, 30m, 45m, 60m
                    let intervals = [(35, 15), (140, 30), (245, 45), (350, 60)];
                    for (bx, val) in intervals {
                        if x >= bx && x < bx + 95 && y >= 65 && y < 65 + 30 {
                            state.config.interval_minutes = val;
                            state.changed = true;
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
                        if x >= bx && x < bx + 130 && y >= 135 && y < 135 + 30 {
                            state.config.position = pos_str.to_string();
                            state.changed = true;
                            needs_redraw = true;
                        }
                    }

                    // Formats: 12-Hour (35, w=200), 24-Hour (245, w=200)
                    if x >= 35 && x < 235 && y >= 205 && y < 205 + 30 {
                        state.config.time_format = "12h".to_string();
                        state.changed = true;
                        needs_redraw = true;
                    } else if x >= 245 && x < 445 && y >= 205 && y < 205 + 30 {
                        state.config.time_format = "24h".to_string();
                        state.changed = true;
                        needs_redraw = true;
                    }

                    // Save Button (320, 265, w=130, h=30)
                    if x >= 310 && x < 450 && y >= 255 && y < 290 {
                        state.closed = true;
                        let _ = DestroyWindow(hwnd);
                        PostQuitMessage(0);
                        return LRESULT(0);
                    }

                    if needs_redraw {
                        render_and_update(hwnd, &state.config);
                    }
                }
            }
        }
        LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn render_settings_buffer(cfg: &Config) -> PixelBuffer {
    let width = 480;
    let height = 310;
    let mut buf = PixelBuffer::new(width, height, 0x00000000);

    let col_bg = 0xFF000000;
    let col_white = 0xFFFFFFFF;
    let col_btn_bg = 0xFF141414;
    let col_btn_border = 0xFF444444;
    let col_muted = 0xFF888888;
    let col_black = 0xFF000000;

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
        buf.draw_text(&text_with_mark, bx + 16, 74, 1, text_col);
    }

    // 2. POSITION SECTION
    buf.draw_text("SCREEN POSITION:", 25, 118, 1, col_muted);
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
        buf.draw_rounded_rect(bx, 135, 130, 30, 6, bg, border, 1);
        buf.draw_text(label, bx + 14, 144, 1, text_col);
    }

    // 3. CLOCK FORMAT SECTION
    buf.draw_text("CLOCK FORMAT:", 25, 188, 1, col_muted);
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
        buf.draw_rounded_rect(bx, 205, bw, 30, 6, bg, border, 1);
        buf.draw_text(label, bx + 16, 214, 1, text_col);
    }

    // Divider
    for px in 20..460 {
        buf.set_pixel(px, 248, col_btn_border);
    }

    // Save Button
    buf.draw_rounded_rect(320, 258, 135, 32, 6, col_white, col_white, 1);
    buf.draw_text("SAVE CONFIG", 336, 269, 1, col_black);

    buf
}

fn render_and_update(hwnd: HWND, cfg: &Config) {
    let buf = render_settings_buffer(cfg);
    unsafe {
        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(screen_dc);

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
        let bitmap = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();

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

pub fn show_settings_dialog(current_cfg: &Config) -> Option<Config> {
    let class_name = w!("TimeReminderSettingsClass");
    let state = Arc::new(Mutex::new(SettingsState {
        config: current_cfg.clone(),
        changed: false,
        closed: false,
    }));

    if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
        *g = Some(state.clone());
    }

    unsafe {
        let wc = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(settings_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap().into(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: windows::core::PCWSTR::null(),
            lpszClassName: class_name,
        };

        RegisterClassW(&wc);

        let width = 480;
        let height = 310;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - width) / 2;
        let y = (screen_h - height) / 2;

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED,
            class_name,
            w!("Time Reminder Settings"),
            WS_POPUP,
            x,
            y,
            width,
            height,
            None,
            None,
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap(),
            None,
        ).unwrap();

        render_and_update(hwnd, current_cfg);
        let _ = ShowWindow(hwnd, SW_SHOW);

        // Run modal message loop
        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }

    if let Ok(mut g) = GLOBAL_SETTINGS_STATE.lock() {
        *g = None;
    }

    let state = state.lock().unwrap();
    if state.changed {
        Some(state.config.clone())
    } else {
        None
    }
}
