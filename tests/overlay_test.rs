#[test]
fn test_overlay_creation_and_reposition() {
    let overlay = time_reminder::overlay::OverlayWindow::new("top-right", 4);
    assert!(overlay.is_ok());
    let overlay = overlay.unwrap();
    overlay.reposition();
}
