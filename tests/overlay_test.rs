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
    assert!(overlay.is_visible());
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
    assert!(!overlay.is_visible());
    assert!(!overlay.is_animating());
}

#[test]
fn test_overlay_preview_lifecycle() {
    let mut overlay = OverlayWindow::new("top-right", 4).expect("Failed to create overlay");
    assert_eq!(overlay.anim_state(), AnimationState::Hidden);
    assert!(!overlay.is_visible());
    assert!(!overlay.is_animating());

    let (x, y, _) = overlay.calculate_positions();
    println!("Initial calculate_positions: x={}, y={}", x, y);

    // Show preview: must be in Preview state, visible, not animating (stationary)
    overlay.show_preview("10:45", "PM", "01H 30M", "top-left");
    assert_eq!(overlay.anim_state(), AnimationState::Preview);
    assert!(overlay.is_visible());
    assert!(!overlay.is_animating()); // Preview is stationary, 0% CPU ticks

    let (x, y, _) = overlay.calculate_positions();
    println!("After top-left show_preview: x={}, y={}", x, y);

    // update() returns false (zero timer ticks needed during preview)
    let needs_anim = overlay.update();
    assert!(!needs_anim);

    // Position updates dynamically
    overlay.show_preview("22:45", "", "02H 15M", "top-center");
    assert_eq!(overlay.anim_state(), AnimationState::Preview);
    assert!(overlay.is_visible());
    assert!(!overlay.is_animating());

    let (x, y, _) = overlay.calculate_positions();
    println!("After top-center show_preview: x={}, y={}", x, y);

    // Hide preview: transitions to Hidden
    overlay.hide();
    assert_eq!(overlay.anim_state(), AnimationState::Hidden);
    assert!(!overlay.is_visible());
    assert!(!overlay.is_animating());
}
