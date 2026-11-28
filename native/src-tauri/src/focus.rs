//! Remember which window had focus before draw mode, and give focus back afterwards.

/// An opaque handle to whatever had focus: a window handle on Windows, the process id of the
/// frontmost app on macOS.
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

/// macOS activates apps, not windows: remember the frontmost app and bring it back. Its
/// previously focused window comes back with it.
#[cfg(target_os = "macos")]
pub fn foreground() -> Option<WindowHandle> {
    use objc2_app_kit::NSWorkspace;
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(WindowHandle(app.processIdentifier() as isize))
}

#[cfg(target_os = "macos")]
pub fn restore(handle: WindowHandle) {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    // An app that has quit since is simply not found.
    if let Some(app) =
        NSRunningApplication::runningApplicationWithProcessIdentifier(handle.0 as i32)
    {
        app.activateWithOptions(NSApplicationActivationOptions::empty());
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn foreground() -> Option<WindowHandle> {
    None
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn restore(_handle: WindowHandle) {}
