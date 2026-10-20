use std::time::Duration;

#[test]
fn test_session_formatting_hours_and_minutes() {
    let d = Duration::from_secs(3600 + 30 * 60); // 1h 30m
    assert_eq!(time_reminder::detector::format_session(d), "01H 30M");
}

#[test]
fn test_session_formatting_zero() {
    let d = Duration::from_secs(0);
    assert_eq!(time_reminder::detector::format_session(d), "00H 00M");
}

#[test]
fn test_session_tracker_pause_and_resume() {
    let mut tracker = time_reminder::detector::SessionTracker::new();
    tracker.tick(true, Duration::from_secs(60));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 60);

    // Game paused/minimized (is_playing = false) for 120s
    tracker.tick(false, Duration::from_secs(120));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 60);

    // Resumed
    tracker.tick(true, Duration::from_secs(30));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 90);

    // Reset
    tracker.reset();
    assert_eq!(tracker.elapsed_playtime().as_secs(), 0);
}

#[test]
fn test_session_tracker_resets_when_game_process_quits() {
    let mut tracker = time_reminder::detector::SessionTracker::new();
    // Simulate game running with PID 999999 (which is not a running process)
    tracker.tick_with_pid(true, Some(999999), Duration::from_secs(300));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 300);

    // Player quits game -> is_playing is false -> game process is dead -> resets session
    tracker.tick_with_pid(false, None, Duration::from_secs(1));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 0);
}

#[test]
fn test_session_tracker_resets_on_extended_inactivity() {
    let mut tracker = time_reminder::detector::SessionTracker::new();
    tracker.tick(true, Duration::from_secs(120));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 120);

    // Away for 301 seconds without PID -> triggers 5-minute inactivity reset
    tracker.tick(false, Duration::from_secs(301));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 0);
}
