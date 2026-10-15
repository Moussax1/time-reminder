use std::env;
use std::path::PathBuf;
use std::time::Duration;
use time_reminder::config::{load_config, save_config, Config};
use time_reminder::detector::{format_session, is_game_fullscreen, SessionTracker};
use time_reminder::overlay::OverlayWindow;
use time_reminder::renderer::format_clock;
use time_reminder::settings_gui::show_settings_dialog;
use time_reminder::tray::{
    TrayIcon, CMD_EXIT, CMD_INTERVAL_15, CMD_INTERVAL_30, CMD_INTERVAL_45, CMD_INTERVAL_60,
    CMD_RESET_SESSION, CMD_SETTINGS, CMD_TEST_NOTIF, WM_TRAYICON,
};
use windows::core::w;
use windows::Win32::Foundation::{GENERIC_ALL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::StationsAndDesktops::{
    OpenDesktopW, OpenWindowStationW, SetProcessWindowStation, SetThreadDesktop,
    DESKTOP_CONTROL_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    KillTimer, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetTimer,
    ShowWindow, TranslateMessage, HCURSOR, HICON, SW_SHOW, WM_DESTROY, WM_HOTKEY,
    WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

static mut MAIN_HWND: Option<HWND> = None;

fn get_config_path() -> PathBuf {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            return dir.join("config.json");
        }
    }
    PathBuf::from("config.json")
}

unsafe extern "system" fn main_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn trigger_time_reminder(overlay: &mut OverlayWindow, tracker: &SessionTracker, config: &Config) {
    // Windows local time via SystemTime conversion
    let local = unsafe {
        windows::Win32::System::SystemInformation::GetLocalTime()
    };

    let (time_str, ampm_str) = format_clock(
        local.wHour as u32,
        local.wMinute as u32,
        &config.time_format,
    );
    let session_str = format_session(tracker.elapsed_playtime());

    overlay.trigger(&time_str, &ampm_str, &session_str);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config_path = get_config_path();
    let mut config = load_config(&config_path);

    // Test overlay CLI mode
    if args.iter().any(|a| a == "--test-overlay") {
        println!("Flashing test overlay for 4 seconds...");
        let mut overlay = OverlayWindow::new(&config.position, 4).expect("Failed to create overlay");
        overlay.trigger("10:45", "PM", "01H 30M");
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(4) {
            overlay.update();
            std::thread::sleep(Duration::from_millis(50));
        }
        println!("Test overlay completed.");
        return;
    }

    // Direct Settings CLI mode
    if args.iter().any(|a| a == "--settings") {
        if let Some(new_cfg) = show_settings_dialog(&config) {
            let _ = save_config(&config_path, &new_cfg);
            println!("Settings saved.");
        }
        return;
    }

    println!("Starting Time Reminder daemon...");

    unsafe {
        // Ensure process and thread are attached to interactive Window Station & Desktop
        if let Ok(winsta) = OpenWindowStationW(w!("WinSta0"), false, GENERIC_ALL.0) {
            let _ = SetProcessWindowStation(winsta);
            if let Ok(desktop) = OpenDesktopW(w!("Default"), DESKTOP_CONTROL_FLAGS(0), false, GENERIC_ALL.0) {
                let _ = SetThreadDesktop(desktop);
            }
        }

        let class_name = w!("TimeReminderDaemonClass");
        let wc = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(main_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap().into(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: windows::core::PCWSTR::null(),
            lpszClassName: class_name,
        };

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name,
            w!("Time Reminder Hidden Window"),
            WS_POPUP,
            -10000,
            -10000,
            1,
            1,
            None,
            None,
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap(),
            None,
        ).unwrap();

        // Must show the window (placed far offscreen) so IsWindowVisible(hwnd) is TRUE.
        // This is strictly required for SetForegroundWindow(hwnd) and TrackPopupMenu to work in Windows 10/11.
        let _ = ShowWindow(hwnd, SW_SHOW);

        MAIN_HWND = Some(hwnd);

        // Register Global Hotkey Alt + T (0x54 = 'T')
        let hotkey_registered = RegisterHotKey(hwnd, 1, MOD_ALT, 0x54).is_ok();
        if hotkey_registered {
            println!("Global hotkey Alt+T registered.");
        }

        let wm_taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        let tray = TrayIcon::new(hwnd).expect("Failed to initialize system tray icon");
        let mut overlay = OverlayWindow::new(&config.position, config.display_duration_secs)
            .expect("Failed to create overlay window");
        let mut tracker = SessionTracker::new();

        // 1-second background tick timer
        SetTimer(hwnd, 100, 1000, None);

        let mut last_reminder_sec: u64 = 0;

        // Flash once on startup to show the player it's working!
        trigger_time_reminder(&mut overlay, &tracker, &config);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_TIMER && msg.wParam.0 == 100 {
                let playing = is_game_fullscreen();
                tracker.tick(playing, Duration::from_secs(1));

                let elapsed_secs = tracker.elapsed_playtime().as_secs();
                let interval_secs = (config.interval_minutes as u64) * 60;

                // Check interval trigger (only when playing and interval reached)
                if playing && interval_secs > 0 && elapsed_secs > 0 && elapsed_secs % interval_secs == 0 {
                    if elapsed_secs != last_reminder_sec {
                        last_reminder_sec = elapsed_secs;
                        trigger_time_reminder(&mut overlay, &tracker, &config);
                    }
                }

                overlay.update();
            } else if msg.message == WM_HOTKEY {
                // Alt + T pressed
                trigger_time_reminder(&mut overlay, &tracker, &config);
            } else if msg.message == WM_TRAYICON {
                let event = (msg.lParam.0 & 0xFFFF) as u32;
                // Support Right-click (WM_RBUTTONUP 0x0205, WM_RBUTTONDOWN 0x0204), Left-click (WM_LBUTTONUP 0x0202), and WM_CONTEXTMENU (0x007B)
                if event == WM_RBUTTONUP || event == 0x0204 || event == WM_LBUTTONUP || event == 0x007B {
                    let cmd = tray.show_menu(hwnd, config.interval_minutes);
                    match cmd {
                        CMD_INTERVAL_15 => {
                            config.interval_minutes = 15;
                            let _ = save_config(&config_path, &config);
                        }
                        CMD_INTERVAL_30 => {
                            config.interval_minutes = 30;
                            let _ = save_config(&config_path, &config);
                        }
                        CMD_INTERVAL_45 => {
                            config.interval_minutes = 45;
                            let _ = save_config(&config_path, &config);
                        }
                        CMD_INTERVAL_60 => {
                            config.interval_minutes = 60;
                            let _ = save_config(&config_path, &config);
                        }
                        CMD_SETTINGS => {
                            if let Some(new_cfg) = show_settings_dialog(&config) {
                                config = new_cfg;
                                let _ = save_config(&config_path, &config);
                                overlay = OverlayWindow::new(
                                    &config.position,
                                    config.display_duration_secs,
                                ).expect("Failed to update overlay");
                            }
                        }
                        CMD_TEST_NOTIF => {
                            trigger_time_reminder(&mut overlay, &tracker, &config);
                        }
                        CMD_RESET_SESSION => {
                            tracker.reset();
                            last_reminder_sec = 0;
                            trigger_time_reminder(&mut overlay, &tracker, &config);
                        }
                        CMD_EXIT => {
                            let _ = DestroyWindow(hwnd);
                            break;
                        }
                        _ => {}
                    }
                } else if event == 0x0203 /* WM_LBUTTONDBLCLK */ {
                    if let Some(new_cfg) = show_settings_dialog(&config) {
                        config = new_cfg;
                        let _ = save_config(&config_path, &config);
                        overlay = OverlayWindow::new(
                            &config.position,
                            config.display_duration_secs,
                        ).expect("Failed to update overlay");
                    }
                }
            } else if wm_taskbar_created != 0 && msg.message == wm_taskbar_created {
                let _ = tray.re_add();
            }

            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }

        let _ = KillTimer(hwnd, 100);
        let _ = UnregisterHotKey(hwnd, 1);
        drop(tray);
    }

    println!("Time Reminder daemon stopped cleanly.");
}
