use time_reminder::tray::create_clock_icon;
use windows::Win32::Graphics::Gdi::{DeleteObject, GetObjectW, BITMAP};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

#[test]
fn test_clock_icon_mask_and_dimensions() {
    unsafe {
        let hicon = create_clock_icon();
        assert!(!hicon.is_invalid(), "create_clock_icon returned invalid icon");

        let mut info = ICONINFO::default();
        let res = GetIconInfo(hicon, &mut info);
        assert!(res.is_ok(), "GetIconInfo failed");

        let mut bmp_mask = BITMAP::default();
        let size = GetObjectW(
            windows::Win32::Graphics::Gdi::HGDIOBJ(info.hbmMask.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp_mask as *mut _ as _),
        );
        assert_eq!(size, std::mem::size_of::<BITMAP>() as i32);
        assert_eq!(bmp_mask.bmWidth, 16);
        assert_eq!(bmp_mask.bmHeight, 16);

        let _ = DeleteObject(info.hbmMask);
        if !info.hbmColor.is_invalid() {
            let _ = DeleteObject(info.hbmColor);
        }
        let _ = DestroyIcon(hicon);
    }
}

#[test]
fn test_tray_registration_and_v4_mode() {
    use windows::core::w;
    use windows::Win32::Foundation::GENERIC_ALL;
    use windows::Win32::System::StationsAndDesktops::{
        OpenDesktopW, OpenWindowStationW, SetProcessWindowStation, SetThreadDesktop,
        DESKTOP_CONTROL_FLAGS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, RegisterClassW, DefWindowProcW,
        WNDCLASSW, WS_OVERLAPPEDWINDOW,
    };
    use time_reminder::tray::TrayIcon;

    unsafe extern "system" fn dummy_wndproc(
        hwnd: windows::Win32::Foundation::HWND,
        msg: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
    ) -> windows::Win32::Foundation::LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe {
        if let Ok(winsta) = OpenWindowStationW(w!("WinSta0"), false, GENERIC_ALL.0) {
            let _ = SetProcessWindowStation(winsta);
            if let Ok(desktop) = OpenDesktopW(w!("Default"), DESKTOP_CONTROL_FLAGS(0), false, GENERIC_ALL.0) {
                let _ = SetThreadDesktop(desktop);
            }
        }

        let class_name = w!("TestTrayRegClass");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(dummy_wndproc),
            hInstance: windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap().into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            class_name,
            w!("TestTrayCarrier"),
            WS_OVERLAPPEDWINDOW,
            0, 0, 0, 0,
            None, None,
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap(),
            None,
        ).unwrap();

        let mut tray = TrayIcon::new(hwnd).expect("TrayIcon::new failed");
        assert!(tray.re_add(), "TrayIcon::re_add failed");
        drop(tray);
        let _ = DestroyWindow(hwnd);
    }
}
