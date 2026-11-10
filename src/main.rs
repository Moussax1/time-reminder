#![windows_subsystem = "windows"]

use std::env;
use std::path::PathBuf;
use std::time::Duration;
use time_reminder::config::{load_config, save_config, Config};
use time_reminder::detector::{
    detect_game_fullscreen_filtered, format_session, SessionTracker, IDLE_ACTIVE_THRESHOLD,
};
use time_reminder::enable_dpi_awareness;
use time_reminder::overlay::OverlayWindow;
use time_reminder::renderer::format_clock;
use time_reminder::settings_gui::{show_settings_dialog, SettingsAction};
use time_reminder::tray::{
    log_debug, TrayIcon, CMD_EXIT, CMD_INTERVAL_15, CMD_INTERVAL_30, CMD_INTERVAL_45,
    CMD_INTERVAL_60, CMD_RESET_SESSION, CMD_SETTINGS, CMD_TEST_NOTIF, NIN_SELECT, WM_TRAYICON,
};
use windows::core::w;
use windows::Win32::Foundation::{GENERIC_ALL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::StationsAndDesktops::{
    OpenDesktopW, OpenWindowStationW, SetProcessWindowStation, SetThreadDesktop,
    DESKTOP_CONTROL_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    SetTimer, TranslateMessage, HCURSOR, HICON, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_HOTKEY,
    WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

const WM_APP_TRAY_GUI: u32 = WM_APP + 200;
const WM_APP_TRAY_MENU: u32 = WM_APP + 201;
const WM_APP_RE_ADD_TRAY: u32 = WM_APP + 202;

static WM_TASKBAR_CREATED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

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
            log_debug("main_wndproc: WM_DESTROY");
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            let id = ((lparam.0 >> 16) & 0xFFFF) as u32;
            log_debug(&format!(
                "main_wndproc: WM_TRAYICON event=0x{:04X} ({}), id={}",
                event, event, id
            ));

            // Differentiate Right-Click (Context Menu) vs Left-Click (Retro Settings GUI)
            // Under NOTIFYICON_VERSION_4, right-click sends WM_CONTEXTMENU and left-click sends NIN_SELECT.
            if event == WM_CONTEXTMENU {
                log_debug("main_wndproc: Right-click received -> posting WM_APP_TRAY_MENU");
                let _ = SetForegroundWindow(hwnd);
                let _ = PostMessageW(hwnd, WM_APP_TRAY_MENU, wparam, lparam);
            } else if event == NIN_SELECT {
                log_debug("main_wndproc: Left-click received -> posting WM_APP_TRAY_GUI");
                let _ = SetForegroundWindow(hwnd);
                let _ = PostMessageW(
                    hwnd,
                    WM_APP_TRAY_GUI,
                    windows::Win32::Foundation::WPARAM(0),
                    lparam,
                );
            }
            LRESULT(0)
        }
        _ => {
            let taskbar_msg = WM_TASKBAR_CREATED.load(std::sync::atomic::Ordering::Relaxed);
            if taskbar_msg != 0 && msg == taskbar_msg {
                log_debug("main_wndproc: TaskbarCreated broadcast received from Explorer -> posting WM_APP_RE_ADD_TRAY");
                let _ = PostMessageW(hwnd, WM_APP_RE_ADD_TRAY, wparam, lparam);
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

fn trigger_time_reminder(overlay: &mut OverlayWindow, tracker: &SessionTracker, config: &Config) {
    // Windows local time via SystemTime conversion
    // SAFETY: GetLocalTime safely queries current local time into a SYSTEMTIME structure.
    let local = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };

    let (time_str, ampm_str) = format_clock(
        local.wHour as u32,
        local.wMinute as u32,
        &config.time_format,
    );
    let session_str = format_session(tracker.elapsed_playtime());

    overlay.trigger(&time_str, &ampm_str, &session_str);
}

fn main() {
    // Ensure the daemon thread is attached to the interactive desktop (WinSta0\Default)
    // even when launched from Task Scheduler, startup scripts, or background processes.
    unsafe {
        if let Ok(winsta) = OpenWindowStationW(w!("WinSta0"), false, GENERIC_ALL.0) {
            let _ = SetProcessWindowStation(winsta);
            if let Ok(desktop) = OpenDesktopW(
                w!("Default"),
                DESKTOP_CONTROL_FLAGS(0),
                false,
                GENERIC_ALL.0,
            ) {
                let _ = SetThreadDesktop(desktop);
            }
        }
    }
    enable_dpi_awareness();
    time_reminder::tray::enable_dark_mode_menu();
    let args: Vec<String> = env::args().collect();
    let config_path = get_config_path();
    let mut config = load_config(&config_path);

    // Clean CLI exit flag
    if args.iter().any(|a| a == "--exit" || a == "--quit") {
        unsafe {
            if let Ok(existing_hwnd) = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                w!("TimeReminderDaemonClass"),
                None,
            ) {
                if !existing_hwnd.is_invalid() {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        existing_hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                    println!("Sent exit signal to running Time Reminder daemon.");
                    return;
                }
            }
        }
        println!("No running Time Reminder daemon found.");
        return;
    }

    // Enforce single instance: if another instance is already running, restore tray icon, signal it to open settings and exit
    // (Exempt --test-overlay so animation tests can run freely without interrupting the background daemon)
    if !args.iter().any(|a| a == "--test-overlay") {
        unsafe {
            if let Ok(existing_hwnd) = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                w!("TimeReminderDaemonClass"),
                None,
            ) {
                if !existing_hwnd.is_invalid() {
                    println!("Time Reminder daemon is already running. Restoring tray icon and focusing settings on active instance.");
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        existing_hwnd,
                        WM_APP_TRAY_GUI,
                        windows::Win32::Foundation::WPARAM(1),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                    return;
                }
            }
        }
    }

    // Test overlay CLI mode
    if args.iter().any(|a| a == "--test-overlay") {
        println!("Flashing test overlay with slide animation...");
        let mut overlay =
            OverlayWindow::new(&config.position, 4).expect("Failed to create overlay");
        overlay.trigger("10:45", "PM", "01H 30M");
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            overlay.update();
            std::thread::sleep(Duration::from_millis(16));
        }
        println!("Test overlay completed.");
        return;
    }

    // Direct Settings CLI mode
    if args.iter().any(|a| a == "--settings") {
        match show_settings_dialog(&config, None) {
            SettingsAction::Save(new_cfg) => {
                let _ = time_reminder::config::set_run_on_startup(new_cfg.run_on_startup);
                let _ = save_config(&config_path, &new_cfg);
                println!("Settings saved.");
            }
            SettingsAction::ResetSession(new_cfg) => {
                let _ = time_reminder::config::set_run_on_startup(new_cfg.run_on_startup);
                let _ = save_config(&config_path, &new_cfg);
                println!("Settings saved and session timer reset.");
            }
            SettingsAction::ExitApp => {
                println!("Exit requested.");
            }
            SettingsAction::Cancel => {
                println!("Settings cancelled.");
            }
        }
        return;
    }

    println!("Starting Time Reminder daemon...");

    // SAFETY: Initializes the Win32 message-only daemon window,
    // global hotkey, system tray icon, and tick timer.
    unsafe {
        let hinstance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
            Ok(h) => h,
            Err(_) => return,
        };

        let class_name = w!("TimeReminderDaemonClass");
        let wc = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(main_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszMenuName: windows::core::PCWSTR::null(),
            lpszClassName: class_name,
        };

        let _ = RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name,
            w!("Time Reminder Daemon"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        ) {
            Ok(h) if !h.is_invalid() => h,
            _ => return,
        };

        // Register Global Hotkey using parsed config with logging
        let (mod_flags, vk) = time_reminder::config::parse_hotkey(&config.hotkey);
        let hotkey_res = RegisterHotKey(
            hwnd,
            1,
            windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS(mod_flags),
            vk,
        );
        match hotkey_res {
            Ok(_) => println!("Global hotkey {} registered.", config.hotkey),
            Err(e) => eprintln!(
                "[WARN] Failed to register global hotkey {}: {:?}. Another application may be using this shortcut.",
                config.hotkey, e
            ),
        }

        let wm_taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        WM_TASKBAR_CREATED.store(wm_taskbar_created, std::sync::atomic::Ordering::Relaxed);
        let mut tray = match TrayIcon::new(hwnd) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to initialize system tray icon: {:?}", e);
                return;
            }
        };
        let mut overlay = match OverlayWindow::new(&config.position, config.display_duration_secs) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Failed to create overlay window: {:?}", e);
                return;
            }
        };
        let mut tracker = SessionTracker::new();

        // 1-second background tick timer
        SetTimer(hwnd, 100, 1000, None);

        let mut last_reminder_sec: u64 = 0;

        // Flash once on startup to show the player it's working!
        trigger_time_reminder(&mut overlay, &tracker, &config);
        overlay.update();
        // Trim working set memory after initialization
        time_reminder::trim_working_set();

        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            if msg.hwnd == overlay.hwnd() {
                overlay.update();
            } else if msg.message == WM_TIMER && msg.wParam.0 == 100 {
                let (playing, game_pid) = detect_game_fullscreen_filtered(
                    &config.ignored_processes,
                    &config.custom_games,
                );
                // Sample user input idle duration once per tick
                let idle_duration = time_reminder::detector::get_idle_duration();
                let is_active = idle_duration < IDLE_ACTIVE_THRESHOLD;

                if playing && is_active {
                    // Actively playing: accumulate 1 second of playtime
                    tracker.tick_with_pid(true, game_pid, Duration::from_secs(1));
                } else if playing && !is_active {
                    // In-game but idle (e.g. cutscene, pause screen, short break):
                    // If AFK for more than 30 minutes, reset session so leaving game overnight doesn't linger
                    if idle_duration >= Duration::from_secs(1800) {
                        tracker.reset();
                    } else {
                        tracker.tick_with_pid(true, game_pid, Duration::ZERO);
                    }
                } else {
                    // Not in a game (e.g. desktop, watching movie, or game closed): advance inactivity towards session reset
                    tracker.tick_with_pid(false, None, Duration::from_secs(1));
                }

                let elapsed_secs = tracker.elapsed_playtime().as_secs();
                let interval_secs = (config.interval_minutes as u64) * 60;

                // Check interval trigger (only when playing actively and interval reached)
                if playing
                    && is_active
                    && interval_secs > 0
                    && elapsed_secs > 0
                    && elapsed_secs.is_multiple_of(interval_secs)
                    && elapsed_secs != last_reminder_sec
                {
                    last_reminder_sec = elapsed_secs;
                    trigger_time_reminder(&mut overlay, &tracker, &config);
                }

                overlay.update();
            } else if msg.message == WM_HOTKEY {
                // Hotkey pressed -> trigger OSD and immediately update first frame
                log_debug("WM_HOTKEY received -> triggering reminder OSD");
                trigger_time_reminder(&mut overlay, &tracker, &config);
                overlay.update();
            } else if msg.message == WM_APP_TRAY_MENU {
                // Right-click -> Context Menu
                log_debug("Displaying tray context menu...");
                let cmd = tray.show_menu(hwnd, config.interval_minutes);
                match cmd {
                    CMD_INTERVAL_15 => {
                        config.interval_minutes = 15;
                        let _ = save_config(&config_path, &config);
                        log_debug("Interval changed to 15m via tray menu.");
                    }
                    CMD_INTERVAL_30 => {
                        config.interval_minutes = 30;
                        let _ = save_config(&config_path, &config);
                        log_debug("Interval changed to 30m via tray menu.");
                    }
                    CMD_INTERVAL_45 => {
                        config.interval_minutes = 45;
                        let _ = save_config(&config_path, &config);
                        log_debug("Interval changed to 45m via tray menu.");
                    }
                    CMD_INTERVAL_60 => {
                        config.interval_minutes = 60;
                        let _ = save_config(&config_path, &config);
                        log_debug("Interval changed to 60m via tray menu.");
                    }
                    CMD_SETTINGS => {
                        let _ = PostMessageW(hwnd, WM_APP_TRAY_GUI, WPARAM(0), LPARAM(0));
                    }
                    CMD_TEST_NOTIF => {
                        log_debug("Test notification triggered via tray menu.");
                        trigger_time_reminder(&mut overlay, &tracker, &config);
                        overlay.update();
                    }
                    CMD_RESET_SESSION => {
                        log_debug("Session timer reset via tray menu.");
                        tracker.reset();
                        trigger_time_reminder(&mut overlay, &tracker, &config);
                        overlay.update();
                    }
                    CMD_EXIT => {
                        log_debug("Exit requested via tray menu.");
                        let _ = DestroyWindow(hwnd);
                        break;
                    }
                    _ => {}
                }
            } else if msg.message == WM_APP_TRAY_GUI {
                // If activated from an external second instance (wParam == 1), ensure tray icon is restored
                if msg.wParam.0 == 1 {
                    let _ = tray.re_add();
                }
                // Left-click -> Retro Settings GUI
                log_debug("Opening 8-Bit Retro Settings GUI...");
                overlay.hide();
                let action = show_settings_dialog(&config, Some(tracker.elapsed_playtime()));
                match action {
                    SettingsAction::Save(new_cfg) => {
                        let old_hotkey = config.hotkey.clone();
                        config = new_cfg;
                        let _ = save_config(&config_path, &config);
                        let _ = time_reminder::config::set_run_on_startup(config.run_on_startup);

                        // If hotkey changed, update registration
                        if config.hotkey != old_hotkey {
                            let _ = UnregisterHotKey(hwnd, 1);
                            let (mod_flags, vk) =
                                time_reminder::config::parse_hotkey(&config.hotkey);
                            let _ = RegisterHotKey(
                                hwnd,
                                1,
                                windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS(
                                    mod_flags,
                                ),
                                vk,
                            );
                        }

                        overlay.set_position(&config.position);
                        log_debug("Settings saved. OSD preview closed.");
                    }
                    SettingsAction::ResetSession(new_cfg) => {
                        config = new_cfg;
                        let _ = save_config(&config_path, &config);
                        log_debug("Session timer reset via Settings GUI.");
                        tracker.reset();
                        trigger_time_reminder(&mut overlay, &tracker, &config);
                        overlay.update();
                    }
                    SettingsAction::ExitApp => {
                        log_debug("ExitApp chosen in Retro Settings GUI. Exiting daemon.");
                        let _ = DestroyWindow(hwnd);
                        break;
                    }
                    SettingsAction::Cancel => {
                        log_debug("Retro Settings GUI cancelled/closed.");
                    }
                }
            } else if msg.message == WM_APP_RE_ADD_TRAY {
                log_debug("Re-adding tray icon after Explorer restart (TaskbarCreated)...");
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
