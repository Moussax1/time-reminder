use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
    NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu,
    GetCursorPos, PostMessageW, SetForegroundWindow, TrackPopupMenu, HICON, ICONINFO, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_NULL,
};

pub const WM_TRAYICON: u32 = WM_APP + 100;
pub const NIN_SELECT: u32 = 0x0400;

pub const CMD_INTERVAL_15: u32 = 1001;
pub const CMD_INTERVAL_30: u32 = 1002;
pub const CMD_INTERVAL_45: u32 = 1003;
pub const CMD_INTERVAL_60: u32 = 1004;
pub const CMD_SETTINGS: u32 = 2001;
pub const CMD_TEST_NOTIF: u32 = 2002;
pub const CMD_RESET_SESSION: u32 = 2003;
pub const CMD_EXIT: u32 = 9999;

pub fn log_debug(msg: &str) {
    #[cfg(debug_assertions)]
    {
        use std::io::Write;
        // SAFETY: GetLocalTime safely queries current local system time into SYSTEMTIME struct.
        let lt = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        let line = format!(
            "[{:02}:{:02}:{:02}.{:03}] {}",
            lt.wHour, lt.wMinute, lt.wSecond, lt.wMilliseconds, msg
        );
        println!("{}", line);
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let log_path = parent.join("tray_debug.log");
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)
                {
                    let _ = writeln!(f, "{}", line);
                }
            }
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = msg;
}

static INIT_DARK_MENU: std::sync::Once = std::sync::Once::new();

pub fn enable_dark_mode_menu() {
    INIT_DARK_MENU.call_once(|| {
        // SAFETY: Dynamically queries uxtheme.dll once to request dark mode menus on Windows 10/11
        // using undocumented ordinal 135 (SetPreferredAppMode) or ordinal 133 (AllowDarkModeForApp).
        // Fails silently on older OS versions without error.
        unsafe {
            if let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) {
                // Ordinal 135: SetPreferredAppMode(2 = ForceDark) for Windows 10 1903+ and Windows 11
                let set_preferred_app_mode: Option<unsafe extern "system" fn(i32) -> i32> =
                    std::mem::transmute(GetProcAddress(
                        uxtheme,
                        windows::core::PCSTR(135_usize as *const u8),
                    ));
                if let Some(func) = set_preferred_app_mode {
                    let _ = func(2); // 2 = ForceDark
                } else {
                    // Ordinal 133: AllowDarkModeForApp(true) for Windows 10 1809
                    let allow_dark_mode_for_app: Option<unsafe extern "system" fn(bool) -> bool> =
                        std::mem::transmute(GetProcAddress(
                            uxtheme,
                            windows::core::PCSTR(133_usize as *const u8),
                        ));
                    if let Some(func) = allow_dark_mode_for_app {
                        let _ = func(true);
                    }
                }

                // Ordinal 136: FlushMenuThemes() triggers Windows to refresh menu themes
                let flush_menu_themes: Option<unsafe extern "system" fn()> = std::mem::transmute(
                    GetProcAddress(uxtheme, windows::core::PCSTR(136_usize as *const u8)),
                );
                if let Some(func) = flush_menu_themes {
                    func();
                }
            }
        }
    });
}

pub struct TrayIcon {
    nid: NOTIFYICONDATAW,
    hicon: HICON,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> windows::core::Result<Self> {
        enable_dark_mode_menu();
        let hicon = load_or_create_tray_icon();

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: hicon,
            szTip: [0; 128],
            ..Default::default()
        };

        let tip = "Time Reminder (Gaming HUD)";
        for (i, c) in tip.encode_utf16().enumerate() {
            if i < 127 {
                nid.szTip[i] = c;
            }
        }

        // SAFETY: Shell_NotifyIconW registers the tray icon with the Windows shell taskbar notification area.
        // nid is correctly populated with cbSize, hwnd, callback message, icon handle, and tooltip string.
        unsafe {
            let added = Shell_NotifyIconW(NIM_ADD, &nid).as_bool();
            if added {
                log_debug(&format!(
                    "Shell_NotifyIconW NIM_ADD succeeded (cbSize: {})",
                    nid.cbSize
                ));
                nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                let ver_res = Shell_NotifyIconW(NIM_SETVERSION, &nid);
                log_debug(&format!(
                    "Shell_NotifyIconW NIM_SETVERSION(4) returned {}",
                    ver_res.as_bool()
                ));
            } else {
                let err = windows::Win32::Foundation::GetLastError();
                log_debug(&format!(
                    "[WARN] Shell_NotifyIconW NIM_ADD failed. LastError: {:?}",
                    err
                ));
            }
        }

        Ok(Self { nid, hicon })
    }

    pub fn re_add(&mut self) -> bool {
        // SAFETY: Re-registers tray icon with explorer if explorer.exe restarts (TaskbarCreated message).
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            let ok = Shell_NotifyIconW(NIM_ADD, &self.nid).as_bool();
            if ok {
                self.nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                let _ = Shell_NotifyIconW(NIM_SETVERSION, &self.nid);
                log_debug("TrayIcon re-added successfully");
            }
            ok
        }
    }

    pub fn show_menu(&self, hwnd: HWND, active_interval: u32) -> u32 {
        enable_dark_mode_menu();
        // SAFETY: Creates popup context menu, attaches items, displays it at current mouse coordinates,
        // and destroys the menu upon return to prevent handle leakage.
        unsafe {
            log_debug("Creating popup menu...");
            let hmenu = match CreatePopupMenu() {
                Ok(m) if !m.is_invalid() => m,
                _ => return 0,
            };

            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                CMD_INTERVAL_15 as usize,
                w!("Every 15 Minutes"),
            );
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                CMD_INTERVAL_30 as usize,
                w!("Every 30 Minutes"),
            );
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                CMD_INTERVAL_45 as usize,
                w!("Every 45 Minutes"),
            );
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                CMD_INTERVAL_60 as usize,
                w!("Every 60 Minutes"),
            );

            // Check active interval
            let active_cmd = match active_interval {
                15 => CMD_INTERVAL_15,
                45 => CMD_INTERVAL_45,
                60 => CMD_INTERVAL_60,
                _ => CMD_INTERVAL_30,
            };
            let _ = CheckMenuItem(hmenu, active_cmd, MF_CHECKED.0);

            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_SETTINGS as usize, w!("Settings..."));
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                CMD_TEST_NOTIF as usize,
                w!("Test Notification (Alt+T)"),
            );
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                CMD_RESET_SESSION as usize,
                w!("Reset Session Timer"),
            );
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_EXIT as usize, w!("Exit"));

            let mut pt = POINT { x: 0, y: 0 };
            let _ = GetCursorPos(&mut pt);
            log_debug(&format!("show_menu: cursor pos=({}, {})", pt.x, pt.y));

            let fg = SetForegroundWindow(hwnd);
            log_debug(&format!("show_menu: SetForegroundWindow returned {:?}", fg));

            let cmd = TrackPopupMenu(
                hmenu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                pt.x,
                pt.y,
                0,
                hwnd,
                None,
            );
            let err = windows::Win32::Foundation::GetLastError();
            log_debug(&format!(
                "show_menu: TrackPopupMenu returned cmd={}, LastError={:?}",
                cmd.0, err
            ));

            let _ = DestroyMenu(hmenu);
            let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            cmd.0 as u32
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        // SAFETY: Destroys the shell tray icon and the owned HICON when TrayIcon goes out of scope.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            let _ = DestroyIcon(self.hicon);
        }
    }
}

/// Loads the embedded 16x16 icon from the executable resource (resource ID 1) if available,
/// or falls back to synthesizing the Concept T2 hourglass icon programmatically.
pub fn load_or_create_tray_icon() -> HICON {
    // SAFETY: Retrieves current process module handle and loads 16x16 icon resource ID 1.
    // The cast of integer 1 to a pointer matches Win32 MAKEINTRESOURCEW convention.
    #[allow(clippy::manual_dangling_ptr)]
    unsafe {
        if let Ok(hinstance) = windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
            if let Ok(hicon) = windows::Win32::UI::WindowsAndMessaging::LoadImageW(
                hinstance,
                windows::core::PCWSTR(1 as *const u16),
                windows::Win32::UI::WindowsAndMessaging::IMAGE_ICON,
                16,
                16,
                windows::Win32::UI::WindowsAndMessaging::LR_DEFAULTCOLOR,
            ) {
                if !hicon.is_invalid() {
                    return HICON(hicon.0);
                }
            }
        }
    }

    create_hourglass_icon()
}

/// Synthesizes the authentic 16x16 Concept T2 hourglass icon in memory.
/// Frame contour: Cyber Mint (#00FF9D)
/// Active countdown spark: Crisp White (#FFFFFF)
pub fn create_hourglass_icon() -> HICON {
    // SAFETY: Allocates 16x16 1bpp mask and 32bpp color bitmaps in memory, synthesizes an HICON
    // via CreateIconIndirect, and immediately deletes temporary bitmaps so only the HICON persists.
    unsafe {
        // 16x16 1bpp AND mask (2 bytes per row, 16 rows = 32 bytes).
        // Bit 0 = OPAQUE (color shows), Bit 1 = TRANSPARENT.
        // Matches the T2 (Curved Glass Bulb) contour with central spark:
        let mask: [u8; 32] = [
            0xFF, 0xFF, // row 0: transparent
            0xFF, 0xFF, // row 1: transparent
            0xE0, 0x07, // row 2: top bulb bar
            0xE0, 0x07, // row 3: top bulb bar
            0xF8, 0x1F, // row 4: upper chamber
            0xF9, 0x9F, // row 5: upper chamber walls
            0xFD, 0xBF, // row 6: neck taper
            0xFE, 0x7F, // row 7: waist & center spark
            0xFE, 0x7F, // row 8: waist & center spark
            0xFD, 0xBF, // row 9: neck taper
            0xF9, 0x9F, // row 10: lower chamber walls
            0xF8, 0x1F, // row 11: lower chamber
            0xE0, 0x07, // row 12: bottom base bar
            0xE0, 0x07, // row 13: bottom base bar
            0xFF, 0xFF, // row 14: transparent
            0xFF, 0xFF, // row 15: transparent
        ];

        let mut color = [0x00u8; 16 * 16 * 4]; // 32-bit BGRA

        for y in 0..16 {
            let row_mask = ((mask[y * 2] as u16) << 8) | (mask[y * 2 + 1] as u16);
            for x in 0..16 {
                let bit = 15 - x;
                let is_opaque = (row_mask & (1 << bit)) == 0;
                if is_opaque {
                    let pixel_idx = (y * 16 + x) * 4;
                    if (x == 7 || x == 8) && (y == 7 || y == 8) {
                        // Crisp white center time grain
                        color[pixel_idx] = 255;
                        color[pixel_idx + 1] = 255;
                        color[pixel_idx + 2] = 255;
                        color[pixel_idx + 3] = 255;
                    } else {
                        // Cyber Mint glass contour (#00FF9D -> B: 157, G: 255, R: 0, A: 255)
                        color[pixel_idx] = 157;
                        color[pixel_idx + 1] = 255;
                        color[pixel_idx + 2] = 0;
                        color[pixel_idx + 3] = 255;
                    }
                }
            }
        }

        let hbm_mask = CreateBitmap(16, 16, 1, 1, Some(mask.as_ptr() as _));
        let hbm_color = CreateBitmap(16, 16, 1, 32, Some(color.as_ptr() as _));

        let icon_info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let icon = CreateIconIndirect(&icon_info).unwrap_or_default();
        let _ = DeleteObject(hbm_mask);
        let _ = DeleteObject(hbm_color);
        icon
    }
}

/// Backwards compatibility alias for create_hourglass_icon.
#[inline]
pub fn create_clock_icon() -> HICON {
    create_hourglass_icon()
}
