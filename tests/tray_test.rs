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
