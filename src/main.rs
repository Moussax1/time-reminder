use std::env;
use std::path::PathBuf;
use std::time::Duration;
use time_reminder::config::{load_config, save_config, Config};
use time_reminder::detector::{detect_game_fullscreen, format_session, SessionTracker};
use time_reminder::overlay::OverlayWindow;
use time_reminder::renderer::format_clock;
use time_reminder::settings_gui::{show_settings_dialog, SettingsAction};
use time_reminder::tray::{log_debug, TrayIcon, NIN_SELECT, WM_TRAYICON};
use windows::core::w;
use windows::Win32::Foundation::{GENERIC_ALL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::StationsAndDesktops::{
    OpenDesktopW, OpenWindowStationW, SetProcessWindowStation, SetThreadDesktop,
    DESKTOP_CONTROL_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
    PeekMessageW, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetTimer,
    TranslateMessage, HCURSOR, HICON, PM_REMOVE, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_HOTKEY,
    WM_TIMER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

const WM_APP_TRAY_TRIGGER: u32 = WM_APP + 200;

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

            // Forward actionable events (WM_CONTEXTMENU, NIN_SELECT, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP)
            // directly to the thread message queue so GetMessageW executes show_settings_dialog
            if event == WM_CONTEXTMENU
                || event == NIN_SELECT
                || event == 0x0203 /* WM_LBUTTONDBLCLK */
                || event == 0x0202 /* WM_LBUTTONUP */
                || event == 0x0205
            /* WM_RBUTTONUP */
            {
                log_debug(&format!(
                    "main_wndproc: Posting WM_APP_TRAY_TRIGGER (event=0x{:04X}) to thread queue",
                    event
                ));
                let _ = PostMessageW(hwnd, WM_APP_TRAY_TRIGGER, wparam, lparam);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn enable_dpi_awareness() {
    // SAFETY: Dynamically queries user32.dll for SetProcessDpiAwarenessContext, which is
    // available on Windows 10/11. Does nothing if not present, preventing crashes on older OS versions.
    unsafe {
        // SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4)
        if let Ok(user32) =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(w!("user32.dll"))
        {
            let proc_name = std::ffi::CString::new("SetProcessDpiAwarenessContext").unwrap();
            if let Some(proc) = windows::Win32::System::LibraryLoader::GetProcAddress(
                user32,
                windows::core::PCSTR(proc_name.as_ptr() as *const u8),
            ) {
                type SetProcessDpiAwarenessContextFn =
                    unsafe extern "system" fn(isize) -> windows::Win32::Foundation::BOOL;
                let func: SetProcessDpiAwarenessContextFn = std::mem::transmute(proc);
                let _ = func(-4);
            }
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
    enable_dpi_awareness();
    let args: Vec<String> = env::args().collect();
    let config_path = get_config_path();
    let mut config = load_config(&config_path);

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
        match show_settings_dialog(&config) {
            SettingsAction::Save(new_cfg) => {
                let _ = time_reminder::config::set_run_on_startup(new_cfg.run_on_startup);
                let _ = save_config(&config_path, &new_cfg);
                println!("Settings saved.");
            }
            SettingsAction::TestNotif(new_cfg) => {
                let _ = time_reminder::config::set_run_on_startup(new_cfg.run_on_startup);
                let _ = save_config(&config_path, &new_cfg);
                println!("Settings saved and test requested.");
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

    // SAFETY: Initializes the Win32 message-only daemon window, desktop attachment,
    // global hotkey, system tray icon, and tick timer.
    unsafe {
        // Ensure process and thread are attached to interactive Window Station & Desktop
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
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            class_name,
            w!("Time Reminder Daemon"),
            WS_OVERLAPPEDWINDOW,
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

        // Register Global Hotkey using parsed config
        let (mod_flags, vk) = time_reminder::config::parse_hotkey(&config.hotkey);
        let hotkey_registered = RegisterHotKey(
            hwnd,
            1,
            windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS(mod_flags),
            vk,
        )
        .is_ok();
        if hotkey_registered {
            println!("Global hotkey {} registered.", config.hotkey);
        }

        let wm_taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
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
        let mut last_flyout_closed: Option<std::time::Instant> = None;

        // Flash once on startup to show the player it's working!
        trigger_time_reminder(&mut overlay, &tracker, &config);
        // Trim working set memory after initialization
        time_reminder::trim_working_set();

        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            if msg.hwnd == overlay.hwnd() {
                overlay.update();
            } else if msg.message == WM_TIMER && msg.wParam.0 == 100 {
                let (playing, game_pid) = detect_game_fullscreen();
                tracker.tick_with_pid(playing, game_pid, Duration::from_secs(1));

                let elapsed_secs = tracker.elapsed_playtime().as_secs();
                let interval_secs = (config.interval_minutes as u64) * 60;

                // Check interval trigger (only when playing and interval reached)
                if playing
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
                // Hotkey pressed
                trigger_time_reminder(&mut overlay, &tracker, &config);
            } else if msg.message == WM_APP_TRAY_TRIGGER || msg.message == WM_TRAYICON {
                let event = (msg.lParam.0 & 0xFFFF) as u32;
                let id = ((msg.lParam.0 >> 16) & 0xFFFF) as u32;
                log_debug(&format!(
                    "GetMessage: Tray trigger event=0x{:04X} ({}), id={}",
                    event, event, id
                ));

                // Drain redundant trigger events queued in the same click action
                let mut discard_msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                while PeekMessageW(
                    &mut discard_msg,
                    hwnd,
                    WM_APP_TRAY_TRIGGER,
                    WM_APP_TRAY_TRIGGER,
                    PM_REMOVE,
                )
                .as_bool()
                {}
                while PeekMessageW(&mut discard_msg, hwnd, WM_TRAYICON, WM_TRAYICON, PM_REMOVE)
                    .as_bool()
                {}

                // If closed within 350ms, this click was the dismiss click on the tray icon itself
                if let Some(t) = last_flyout_closed {
                    if t.elapsed() < Duration::from_millis(350) {
                        log_debug(
                            "Ignoring tray click immediately after flyout close (toggle dismiss)",
                        );
                        continue;
                    }
                }

                log_debug("Directly opening 8-Bit Retro Settings GUI from tray event...");
                let action = show_settings_dialog(&config);
                last_flyout_closed = Some(std::time::Instant::now());

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

                        if let Ok(new_overlay) =
                            OverlayWindow::new(&config.position, config.display_duration_secs)
                        {
                            overlay = new_overlay;
                        }
                        log_debug("Settings saved. Triggering preview notification.");
                        trigger_time_reminder(&mut overlay, &tracker, &config);
                    }
                    SettingsAction::TestNotif(new_cfg) => {
                        config = new_cfg;
                        let _ = save_config(&config_path, &config);
                        if let Ok(new_overlay) =
                            OverlayWindow::new(&config.position, config.display_duration_secs)
                        {
                            overlay = new_overlay;
                        }
                        log_debug("Test OSD triggered from Settings GUI.");
                        trigger_time_reminder(&mut overlay, &tracker, &config);
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
