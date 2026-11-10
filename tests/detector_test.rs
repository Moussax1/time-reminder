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

#[test]
fn test_in_game_idle_pauses_playtime_and_preserves_session() {
    let mut tracker = time_reminder::detector::SessionTracker::new();
    let current_pid = std::process::id();

    // 10 minutes of active playing
    for _ in 0..600 {
        tracker.tick_with_pid(true, Some(current_pid), Duration::from_secs(1));
    }
    assert_eq!(tracker.elapsed_playtime().as_secs(), 600);

    // 10 minutes of in-game cutscene/AFK (idle): zero delta ticked
    for _ in 0..600 {
        tracker.tick_with_pid(true, Some(current_pid), Duration::ZERO);
    }
    // Session is preserved, playtime did not reset to zero!
    assert_eq!(tracker.elapsed_playtime().as_secs(), 600);
    assert!(tracker.is_active());

    // Player resumes active gameplay for 1 minute
    tracker.tick_with_pid(true, Some(current_pid), Duration::from_secs(60));
    assert_eq!(tracker.elapsed_playtime().as_secs(), 660);
}

#[test]
fn test_is_game_path_identifies_standard_game_roots() {
    use time_reminder::detector::is_game_path;

    let game_paths = [
        r"C:\Program Files (x86)\Steam\steamapps\common\Cyberpunk 2077\bin\x64\Cyberpunk2077.exe",
        r"D:\SteamLibrary\steamapps\common\Witcher 3\bin\x64\witcher3.exe",
        r"E:\Games\Elden Ring\Game\eldenring.exe",
        r"C:\XboxGames\Starfield\Content\Starfield.exe",
        r"C:\Program Files\WindowsApps\Microsoft.MinecraftUWP_8wekyb3d8bbwe\Minecraft.Windows.exe",
        r"D:\Epic Games\Fortnite\FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe",
        r"C:\GOG Games\Hollow Knight\hollow_knight.exe",
        r"C:\Program Files (x86)\GOG Galaxy\Games\Baldur's Gate 3\bin\bg3.exe",
        r"C:\Riot Games\VALORANT\live\ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe",
        r"C:\Program Files\Ubisoft\Ubisoft Game Launcher\games\Rainbow Six Siege\RainbowSix.exe",
        r"D:\EA Games\Apex Legends\r5apex.exe",
        r"C:\Program Files (x86)\Battle.net\Diablo IV\Diablo IV.exe",
        r"D:\MyCustom\Games\Balatro\balatro.exe",
    ];

    for path in &game_paths {
        assert!(
            is_game_path(path),
            "Expected game path '{}' to be recognized",
            path
        );
    }
}

#[test]
fn test_is_game_path_rejects_browsers_and_media_players() {
    use time_reminder::detector::is_game_path;

    let non_game_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Mozilla Firefox\firefox.exe",
        r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
        r"C:\Program Files\VideoLAN\VLC\vlc.exe",
        r"C:\Program Files\MPC-HC\mpc-hc64.exe",
        r"C:\Users\User\AppData\Roaming\mpv\mpv.exe",
        r"C:\Program Files\PotPlayer\PotPlayer64.exe",
        r"C:\Program Files\Windows Media Player\wmplayer.exe",
        r"C:\Windows\System32\notepad.exe",
        r"C:\Windows\explorer.exe",
        r"C:\Program Files\Microsoft Office\root\Office16\POWERPNT.EXE",
        r"C:\Users\User\AppData\Local\Programs\Microsoft VS Code\Code.exe",
        r"C:\Users\User\AppData\Local\Discord\app-1.0.9001\Discord.exe",
        r"C:\Program Files\Spotify\Spotify.exe",
        // UWP media/video players located inside WindowsApps directory
        r"C:\Program Files\WindowsApps\4DF9E0F8.Netflix_6.99.5.0_x64__8wekyb3d8bbwe\Netflix.exe",
        r"C:\Program Files\WindowsApps\Microsoft.ZuneVideo_10.22091.10041.0_x64__8wekyb3d8bbwe\Video.UI.exe",
        r"C:\Program Files\WindowsApps\Microsoft.Media.Player_11.2301.27.0_x64__8wekyb3d8bbwe\Microsoft.Media.Player.exe",
        r"C:\Program Files\WindowsApps\SpotifyAB.SpotifyMusic_1.231.1205.0_x86__zpdnekdrzrea0\Spotify.exe",
        r"C:\Program Files\WindowsApps\Disney.37853FC22B2CE_1.13.0.0_x64__6rarf9sa4v8jt\DisneyPlus.exe",
        r"C:\Program Files\WindowsApps\AmazonVideo.PrimeVideo_1.0.115.0_x64__pwbj9vvecjh7j\PrimeVideo.exe",
    ];

    for path in &non_game_paths {
        assert!(
            !is_game_path(path),
            "Expected non-game path '{}' to NOT be recognized as a game",
            path
        );
    }
}

#[test]
fn test_is_desktop_or_shell_window() {
    use time_reminder::detector::is_desktop_or_shell_window;

    assert!(is_desktop_or_shell_window("Progman"));
    assert!(is_desktop_or_shell_window("WorkerW"));
    assert!(is_desktop_or_shell_window("Shell_TrayWnd"));
    assert!(is_desktop_or_shell_window("Shell_SecondaryTrayWnd"));

    assert!(!is_desktop_or_shell_window("UnityWndClass"));
    assert!(!is_desktop_or_shell_window("UnrealWindow"));
    assert!(!is_desktop_or_shell_window("Valve001"));
}

#[test]
fn test_get_process_name_current_process() {
    let pid = std::process::id();
    let name = time_reminder::detector::get_process_name(pid);
    assert!(
        name.is_some(),
        "Current process should have a readable image name"
    );
    let name_str = name.unwrap();
    assert!(
        name_str.ends_with(".exe"),
        "Process name should end with .exe: {}",
        name_str
    );
}

#[test]
fn test_idle_duration_and_activity_check() {
    let idle = time_reminder::detector::get_idle_duration();
    // In any running OS environment, idle duration is non-negative and finite
    assert!(idle.as_secs() < 365 * 24 * 3600);

    // With a generous 1-day threshold, active should return true
    assert!(time_reminder::detector::is_user_input_active(
        Duration::from_secs(86400)
    ));

    // With a 0-second threshold, active should return false (unless input was 0ns ago)
    assert!(!time_reminder::detector::is_user_input_active(
        Duration::ZERO
    ));
}

#[test]
fn test_matches_process_filter() {
    use time_reminder::detector::matches_process_filter;

    let filters = vec![
        "VLC.EXE".to_string(),
        "custom_player.exe".to_string(),
        "D:/Games/balatro.exe".to_string(),
        "".to_string(),
        "   ".to_string(),
    ];

    // Case-insensitive exact match with .EXE stripped
    assert!(matches_process_filter(
        "vlc",
        r"c:\program files\vlc\vlc.exe",
        &filters
    ));
    assert!(matches_process_filter(
        "custom_player",
        r"c:\tools\custom_player.exe",
        &filters
    ));

    // Forward-slash vs back-slash path normalization
    assert!(matches_process_filter(
        "balatro",
        r"d:\games\balatro.exe",
        &filters
    ));

    // Unmatched process
    assert!(!matches_process_filter(
        "notepad",
        r"c:\windows\notepad.exe",
        &filters
    ));
}

#[test]
fn test_detect_game_fullscreen_filtered_signature() {
    // Calling detect_game_fullscreen and detect_game_fullscreen_filtered should not crash
    let _ = time_reminder::detector::detect_game_fullscreen();
    let custom_ignored = vec![
        "VLC.EXE".to_string(),
        "custom_player.exe".to_string(),
        "".to_string(),
        "  ".to_string(),
    ];
    let custom_games = vec![
        "BALATRO.EXE".to_string(),
        "my_game".to_string(),
        "D:/CustomGames/Game/game.exe".to_string(),
        "".to_string(),
    ];
    let _ =
        time_reminder::detector::detect_game_fullscreen_filtered(&custom_ignored, &custom_games);
}
