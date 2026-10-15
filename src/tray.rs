use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreateIconIndirect, CreatePopupMenu, DestroyIcon,
    DestroyMenu, GetCursorPos, PostMessageW, SetForegroundWindow, TrackPopupMenu,
    ICONINFO, MF_CHECKED, MF_SEPARATOR, MF_STRING,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WM_NULL, WM_USER, HICON,
};

pub const WM_TRAYICON: u32 = WM_USER + 1;

pub const CMD_INTERVAL_15: u32 = 1001;
pub const CMD_INTERVAL_30: u32 = 1002;
pub const CMD_INTERVAL_45: u32 = 1003;
pub const CMD_INTERVAL_60: u32 = 1004;
pub const CMD_SETTINGS: u32 = 2001;
pub const CMD_TEST_NOTIF: u32 = 2002;
pub const CMD_RESET_SESSION: u32 = 2003;
pub const CMD_EXIT: u32 = 9999;

pub struct TrayIcon {
    nid: NOTIFYICONDATAW,
    hicon: HICON,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> windows::core::Result<Self> {
        let hicon = create_clock_icon();

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

        unsafe {
            let mut added = false;
            for _ in 0..5 {
                if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                    added = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if !added {
                eprintln!("[WARN] Shell_NotifyIconW NIM_ADD failed to register tray icon");
            }
        }

        Ok(Self { nid, hicon })
    }

    pub fn re_add(&self) -> bool {
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.nid).as_bool()
        }
    }

    pub fn show_menu(&self, hwnd: HWND, active_interval: u32) -> u32 {
        unsafe {
            let hmenu = CreatePopupMenu().unwrap();

            let _ = AppendMenuW(hmenu, MF_STRING, CMD_INTERVAL_15 as usize, w!("Every 15 Minutes"));
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_INTERVAL_30 as usize, w!("Every 30 Minutes"));
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_INTERVAL_45 as usize, w!("Every 45 Minutes"));
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_INTERVAL_60 as usize, w!("Every 60 Minutes"));

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
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_TEST_NOTIF as usize, w!("Test Notification (Alt+T)"));
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_RESET_SESSION as usize, w!("Reset Session Timer"));
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
            let _ = AppendMenuW(hmenu, MF_STRING, CMD_EXIT as usize, w!("Exit"));

            let mut pt = POINT { x: 0, y: 0 };
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(hwnd);

            let cmd = TrackPopupMenu(
                hmenu,
                TPM_RETURNCMD | TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RIGHTBUTTON,
                pt.x,
                pt.y,
                0,
                hwnd,
                None,
            );

            let _ = DestroyMenu(hmenu);
            let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            cmd.0 as u32
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            let _ = DestroyIcon(self.hicon);
        }
    }
}

// Generates a clean 16x16 high-contrast clock icon programmatically
pub fn create_clock_icon() -> HICON {
    unsafe {
        // 16x16 1bpp AND mask: 2 bytes per row, 16 rows = 32 bytes
        // Bit 0 = OPAQUE (color shows), Bit 1 = TRANSPARENT
        let mut mask = [0xFFu8; 32]; // Default all transparent (1)
        let mut color = [0x00u8; 16 * 16 * 4]; // 32-bit BGRA

        // Draw 15px circular clock face centered at (7, 7)
        for y in 0..16 {
            for x in 0..16 {
                let dx = x as i32 - 7;
                let dy = y as i32 - 7;
                let dist_sq = dx * dx + dy * dy;

                let pixel_idx = (y * 16 + x) * 4;
                let mask_byte = y * 2 + (x / 8);
                let mask_bit = 7 - (x % 8);

                if dist_sq <= 49 {
                    // Inside or on the circle: mark pixel as OPAQUE (bit = 0)
                    mask[mask_byte] &= !(1 << mask_bit);

                    if dist_sq >= 36 {
                        // Outer rim: solid black border (RGB 0, 0, 0) for light mode contrast
                        color[pixel_idx] = 0;       // B
                        color[pixel_idx + 1] = 0;   // G
                        color[pixel_idx + 2] = 0;   // R
                        color[pixel_idx + 3] = 255; // A
                    } else {
                        // Inner dial: bright white (RGB 255, 255, 255) for dark mode contrast
                        // Clock hands: pointing up at 12 and right at 3
                        let is_hand = (x == 7 && y >= 3 && y <= 7) || (y == 7 && x >= 7 && x <= 11);
                        if is_hand {
                            // Dark black clock hands
                            color[pixel_idx] = 0;
                            color[pixel_idx + 1] = 0;
                            color[pixel_idx + 2] = 0;
                            color[pixel_idx + 3] = 255;
                        } else {
                            // Bright white face
                            color[pixel_idx] = 255;
                            color[pixel_idx + 1] = 255;
                            color[pixel_idx + 2] = 255;
                            color[pixel_idx + 3] = 255;
                        }
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

        let icon = CreateIconIndirect(&icon_info).unwrap();
        let _ = DeleteObject(hbm_mask);
        let _ = DeleteObject(hbm_color);
        icon
    }
}
