#[test]
fn test_render_settings_buffer_dimensions() {
    let cfg = time_reminder::config::Config::default();
    let buf = time_reminder::settings_gui::render_settings_buffer(&cfg);
    assert_eq!(buf.width, 480);
    assert_eq!(buf.height, 310);
    assert_eq!(buf.data.len(), 480 * 310);
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
