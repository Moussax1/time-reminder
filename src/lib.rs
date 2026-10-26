pub mod config;
pub mod detector;
pub mod font_8bit;
pub mod overlay;
pub mod renderer;
pub mod settings_gui;
pub mod tray;

pub fn version() -> &'static str {
    "0.1.0"
}

/// Trims the process working set to release unneeded physical memory pages back to the OS.
///
/// Passing `usize::MAX` for minimum and maximum working set sizes signals the Windows
/// memory manager to purge inactive pages from the process working set.
pub fn trim_working_set() {
    // SAFETY: GetCurrentProcess returns a valid pseudo-handle for the calling process.
    // Setting both sizes to usize::MAX is documented by Microsoft as the official way
    // to empty the working set without restricting future growth.
    unsafe {
        let handle = windows::Win32::System::Threading::GetCurrentProcess();
        let _ = windows::Win32::System::Threading::SetProcessWorkingSetSize(
            handle,
            usize::MAX,
            usize::MAX,
        );
    }
}

static INIT_DPI: std::sync::Once = std::sync::Once::new();

/// Dynamically queries user32.dll for SetProcessDpiAwarenessContext once per process.
/// Fails silently on older OS versions without unwrap or runtime heap allocation.
pub fn enable_dpi_awareness() {
    INIT_DPI.call_once(|| {
        // SAFETY: Dynamically queries user32.dll for SetProcessDpiAwarenessContext, which is
        // available on Windows 10/11. Does nothing if not present, preventing crashes on older OS versions.
        unsafe {
            // SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4)
            if let Ok(user32) = windows::Win32::System::LibraryLoader::GetModuleHandleW(
                windows::core::w!("user32.dll"),
            ) {
                if let Some(proc) = windows::Win32::System::LibraryLoader::GetProcAddress(
                    user32,
                    windows::core::s!("SetProcessDpiAwarenessContext"),
                ) {
                    type SetProcessDpiAwarenessContextFn =
                        unsafe extern "system" fn(isize) -> windows::Win32::Foundation::BOOL;
                    let func: SetProcessDpiAwarenessContextFn = std::mem::transmute(proc);
                    let _ = func(-4);
                }
            }
        }
    });
}
