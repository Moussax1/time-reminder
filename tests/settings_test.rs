#[test]
fn test_render_settings_buffer_dimensions() {
    let cfg = time_reminder::config::Config::default();
    let buf = time_reminder::settings_gui::render_settings_buffer(&cfg);
    assert_eq!(buf.width, 480);
    assert_eq!(buf.height, 375);
    assert_eq!(buf.data.len(), 480 * 375);
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
        x: 35,
        y: 260,
        w: 200,
        h: 30,
    };
    let boot_off_btn = ButtonRect {
        x: 245,
        y: 260,
        w: 200,
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
    let act_test = SettingsAction::TestNotif(cfg);
    let act_exit = SettingsAction::ExitApp;
    let act_cancel = SettingsAction::Cancel;

    assert_ne!(act_exit, act_cancel);
    assert_ne!(act_save, act_test);
}

#[test]
fn test_export_settings_buffer_image() {
    let cfg = time_reminder::config::Config::default();
    let buf = time_reminder::settings_gui::render_settings_buffer(&cfg);
    let path = r"C:\Users\yasso\.gemini\antigravity\brain\fd0d20ac-bd4a-45a9-804d-c51b3e6bb24c\rendered_settings.bmp";
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
