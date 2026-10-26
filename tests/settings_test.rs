#[test]
fn test_render_settings_buffer_dimensions() {
    let cfg = time_reminder::config::Config::default();
    let buf = time_reminder::settings_gui::render_settings_buffer(&cfg);
    assert_eq!(buf.width, 480);
    assert_eq!(buf.height, 398);
    assert_eq!(buf.data.len(), 480 * 398);
    // Buffer has non-zero pixels drawn
    assert!(buf.data.iter().any(|&p| p != 0));
}

#[test]
fn test_button_rect_contains() {
    let btn = time_reminder::settings_gui::ButtonRect {
        x: 10,
        y: 20,
        w: 50,
        h: 30,
    };
    assert!(btn.contains(10, 20));
    assert!(btn.contains(30, 35));
    assert!(!btn.contains(9, 20));
    assert!(!btn.contains(60, 20));
}

#[test]
fn test_settings_actions_and_hit_testing() {
    use time_reminder::config::Config;
    use time_reminder::settings_gui::{ButtonRect, SettingsAction};

    let boot_on_btn = ButtonRect {
        x: 25,
        y: 260,
        w: 210,
        h: 30,
    };
    let boot_off_btn = ButtonRect {
        x: 245,
        y: 260,
        w: 210,
        h: 30,
    };
    let exit_btn = ButtonRect {
        x: 25,
        y: 325,
        w: 105,
        h: 32,
    };
    let test_btn = ButtonRect {
        x: 140,
        y: 325,
        w: 105,
        h: 32,
    };
    let cancel_btn = ButtonRect {
        x: 255,
        y: 325,
        w: 85,
        h: 32,
    };
    let save_btn = ButtonRect {
        x: 350,
        y: 325,
        w: 105,
        h: 32,
    };

    // Point in boot options
    assert!(boot_on_btn.contains(50, 270));
    assert!(!boot_off_btn.contains(50, 270));
    assert!(boot_off_btn.contains(260, 270));

    // Point in exit button
    assert!(exit_btn.contains(50, 335));
    assert!(!test_btn.contains(50, 335));

    // Point in test button
    assert!(test_btn.contains(180, 335));
    assert!(!save_btn.contains(180, 335));

    // Point in cancel button
    assert!(cancel_btn.contains(290, 335));

    // Point in save button
    assert!(save_btn.contains(400, 335));

    let cfg = Config::default();
    let act_save = SettingsAction::Save(cfg.clone());
    let act_test = SettingsAction::TestNotif(cfg.clone());
    let act_reset = SettingsAction::ResetSession(cfg);
    let act_exit = SettingsAction::ExitApp;
    let act_cancel = SettingsAction::Cancel;

    assert_ne!(act_exit, act_cancel);
    assert_ne!(act_save, act_test);
    assert_ne!(act_reset, act_cancel);
}

#[test]
fn test_export_settings_buffer_image() {
    let cfg = time_reminder::config::Config::default();
    let buf = time_reminder::settings_gui::render_settings_buffer(&cfg);
    let path = std::env::temp_dir().join("time_reminder_rendered_settings.bmp");
    use std::io::Write;
    if let Ok(mut file) = std::fs::File::create(path) {
        let width = buf.width as u32;
        let height = buf.height as u32;
        let file_size = 54 + width * height * 4;
        let mut header = [0u8; 54];
        header[0] = b'B';
        header[1] = b'M';
        header[2..6].copy_from_slice(&file_size.to_le_bytes());
        header[10..14].copy_from_slice(&54u32.to_le_bytes());
        header[14..18].copy_from_slice(&40u32.to_le_bytes());
        header[18..22].copy_from_slice(&(width as i32).to_le_bytes());
        header[22..26].copy_from_slice(&(-(height as i32)).to_le_bytes());
        header[26..28].copy_from_slice(&1u16.to_le_bytes());
        header[28..30].copy_from_slice(&32u16.to_le_bytes());
        let _ = file.write_all(&header);
        for &px in &buf.data {
            let b = (px & 0xFF) as u8;
            let g = ((px >> 8) & 0xFF) as u8;
            let r = ((px >> 16) & 0xFF) as u8;
            let a = ((px >> 24) & 0xFF) as u8;
            let _ = file.write_all(&[b, g, r, a]);
        }
    }
}

#[test]
fn test_settings_usability_and_layout_fixes() {
    use time_reminder::settings_gui::{
        BTN_BOOT_OFF, BTN_BOOT_ON, BTN_CLOSE_X, BTN_RESET_SESSION, BTN_SAVE,
    };

    // 1. Verify BTN_CLOSE_X is pulled inward flush with SAVE button right edge (455..460)
    // and does not overhang past the 460 divider boundary.
    assert_eq!(BTN_CLOSE_X.x, 425);
    assert_eq!(BTN_CLOSE_X.w, 35);
    const { assert!(BTN_CLOSE_X.x + BTN_CLOSE_X.w <= 460) };
    assert!(BTN_CLOSE_X.contains(430, 20));

    // 2. Verify SAVE button geometry and differentiation
    assert_eq!(BTN_SAVE.x, 350);
    assert_eq!(BTN_SAVE.w, 105);
    let right_edge_save = BTN_SAVE.x + BTN_SAVE.w;
    assert_eq!(right_edge_save, 455);

    // 3. Verify unified strict column grid margins on boot toggle buttons (25..455)
    assert_eq!(BTN_BOOT_ON.x, 25);
    assert_eq!(BTN_BOOT_ON.w, 210);
    assert_eq!(BTN_BOOT_OFF.x, 245);
    assert_eq!(BTN_BOOT_OFF.w, 210);
    assert_eq!(BTN_BOOT_OFF.x + BTN_BOOT_OFF.w, 455);

    // 4. Verify rendered buffer contains the Cyber Mint accent color on SAVE
    let cfg = time_reminder::config::Config::default();
    let buf = time_reminder::settings_gui::render_settings_buffer(&cfg);

    let col_accent = 0xFF00FF9D;
    let mut found_accent = false;
    for y in 325..(325 + 32) {
        for x in 350..(350 + 105) {
            let px = buf.data[(y * buf.width as i32 + x) as usize];
            if px == col_accent {
                found_accent = true;
                break;
            }
        }
        if found_accent {
            break;
        }
    }
    assert!(
        found_accent,
        "SAVE button should be styled with Cyber Mint accent color"
    );

    // 5. Verify CANCEL button has bright white text (0xFFFFFFFF)
    let col_white = 0xFFFFFFFF;
    let mut found_cancel_white_text = false;
    for y in 325..(325 + 32) {
        for x in 255..(255 + 85) {
            let px = buf.data[(y * buf.width as i32 + x) as usize];
            if px == col_white {
                found_cancel_white_text = true;
                break;
            }
        }
        if found_cancel_white_text {
            break;
        }
    }
    assert!(
        found_cancel_white_text,
        "CANCEL button should use bright white text"
    );

    // 6. Verify divider lines span exactly x = 25 to x = 455
    let col_divider_bottom = 0xFF444444;
    assert_eq!(buf.data[(38 * buf.width as i32 + 25) as usize], col_white);
    assert_eq!(buf.data[(38 * buf.width as i32 + 455) as usize], col_white);
    assert_eq!(buf.data[(38 * buf.width as i32 + 24) as usize], 0xFF000000); // blank outside
    assert_eq!(buf.data[(38 * buf.width as i32 + 456) as usize], 0xFF000000); // blank outside

    assert_eq!(
        buf.data[(310 * buf.width as i32 + 25) as usize],
        col_divider_bottom
    );
    assert_eq!(
        buf.data[(310 * buf.width as i32 + 455) as usize],
        col_divider_bottom
    );
    assert_eq!(buf.data[(310 * buf.width as i32 + 24) as usize], 0xFF000000);
    assert_eq!(
        buf.data[(310 * buf.width as i32 + 456) as usize],
        0xFF000000
    );

    // 7. Verify Reset Session quick action button geometry
    assert_eq!(BTN_RESET_SESSION.x, 315);
    assert!(BTN_RESET_SESSION.contains(320, 368));
    assert!(BTN_RESET_SESSION.contains(320, 362));

    // 8. Verify brightened unselected button text (#B0B0B0) exists in buffer
    let col_muted = 0xFFB0B0B0;
    let found_muted_text = buf.data.contains(&col_muted);
    assert!(
        found_muted_text,
        "Buffer should contain brightened #B0B0B0 text for legibility"
    );

    // 9. Verify hyphen in "HOTKEY: ALT-T" renders at y = 368 + 3 = 371
    // 'HOTKEY: ALT' has 11 chars. The 12th char is '-' at x = 25 + 11 * 9 = 124.
    // The '-' glyph has 0x7E on row 3 (y = 368 + 3 = 371).
    let hyphen_pixel = buf.data[(371 * buf.width as i32 + 125) as usize];
    assert_eq!(
        hyphen_pixel, col_muted,
        "Hyphen in HOTKEY: ALT-T should render cleanly"
    );
}

#[test]
fn test_calculate_tray_flyout_pos() {
    use time_reminder::settings_gui::calculate_tray_flyout_pos;

    // Single 1080p monitor with standard taskbar (1920 x 1032 work area)
    let (x, y) = calculate_tray_flyout_pos(480, 310, 1920, 1032);
    assert_eq!(x, 1920 - 480 - 12);
    assert_eq!(y, 1032 - 310 - 12);
    assert_eq!(x, 1428);
    assert_eq!(y, 710);

    // Secondary monitor offset (3840 x 1032)
    let (x2, y2) = calculate_tray_flyout_pos(480, 310, 3840, 1032);
    assert_eq!(x2, 3840 - 480 - 12);
    assert_eq!(y2, 1032 - 310 - 12);
}
