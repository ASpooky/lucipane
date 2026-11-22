//! Remember which window had focus before draw mode, and give focus back afterwards.

/// An opaque handle to another application's window.
#[derive(Debug, Clone, Copy)]
pub struct WindowHandle(#[allow(dead_code)] isize);

#[cfg(windows)]
pub fn foreground() -> Option<WindowHandle> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: GetForegroundWindow has no preconditions.
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.is_null()).then_some(WindowHandle(hwnd as isize))
}

#[cfg(windows)]
pub fn restore(handle: WindowHandle) {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
    // SAFETY: a stale handle is fine; SetForegroundWindow just fails.
    unsafe { SetForegroundWindow(handle.0 as _) };
}

// TODO(macOS): reactivate the previous app (NSRunningApplication).
#[cfg(not(windows))]
pub fn foreground() -> Option<WindowHandle> {
    None
}

#[cfg(not(windows))]
pub fn restore(_handle: WindowHandle) {}
