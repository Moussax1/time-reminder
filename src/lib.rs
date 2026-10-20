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
