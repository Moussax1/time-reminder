use std::time::Duration;
use time_reminder::overlay::{AnimationState, OverlayWindow};

#[test]
fn test_overlay_creation_and_reposition() {
    let overlay = OverlayWindow::new("top-right", 4);
    assert!(overlay.is_ok());
    let mut overlay = overlay.unwrap();
    overlay.reposition();
    assert_eq!(overlay.anim_state(), AnimationState::Hidden);
    assert!(!overlay.is_animating());
}

#[test]
fn test_overlay_slide_animation_lifecycle() {
    let mut overlay = OverlayWindow::new("top-right", 1).expect("Failed to create overlay");
    assert_eq!(overlay.anim_state(), AnimationState::Hidden);

    // Trigger overlay notification -> starts SlideIn
    overlay.trigger("10:45", "PM", "01H 30M");
    assert!(overlay.is_animating());
    match overlay.anim_state() {
        AnimationState::SlideIn { .. } => {}
        other => panic!("Expected SlideIn state, found {:?}", other),
    }

    // Step through SlideIn (350ms)
    std::thread::sleep(Duration::from_millis(350));
    overlay.update();
    match overlay.anim_state() {
        AnimationState::Hold { .. } => {}
        other => panic!("Expected Hold state after slide-in, found {:?}", other),
    }

    // Step through Hold (1050ms)
    std::thread::sleep(Duration::from_millis(1050));
    overlay.update();
    match overlay.anim_state() {
        AnimationState::SlideOut { .. } => {}
        other => panic!("Expected SlideOut state after hold, found {:?}", other),
    }

    // Step through SlideOut (300ms)
    std::thread::sleep(Duration::from_millis(300));
    let still_active = overlay.update();
    assert!(!still_active);
    assert_eq!(overlay.anim_state(), AnimationState::Hidden);
    assert!(!overlay.is_animating());
}
