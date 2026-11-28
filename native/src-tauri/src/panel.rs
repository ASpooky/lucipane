//! macOS only shows another app's full-screen Space windows that are non-activating panels,
//! so the overlay window Tauri created becomes one.

use std::ffi::c_void;

use objc2::{ClassType, MainThreadOnly, define_class};
use objc2_app_kit::{NSPanel, NSResponder, NSWindow, NSWindowStyleMask};
use objc2_foundation::NSObject;

define_class!(
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LucipanePanel"]
    struct LucipanePanel;

    impl LucipanePanel {
        /// A borderless panel refuses key status by default; draw mode needs the keyboard.
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }
    }
);

unsafe extern "C" {
    fn object_setClass(object: *mut c_void, class: *const c_void) -> *const c_void;
}

/// Turn `window` into a non-activating panel that stays visible when the app is inactive.
///
/// # Safety
/// `window` must be a live NSWindow, and this must run on the main thread. tao keeps its own
/// state in the window delegate, which the class swap leaves alone.
pub unsafe fn make_panel(window: &NSWindow) {
    unsafe {
        object_setClass(
            window as *const NSWindow as *mut c_void,
            LucipanePanel::class() as *const _ as *const c_void,
        );
        let panel = &*(window as *const NSWindow).cast::<NSPanel>();
        panel.setStyleMask(panel.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        panel.setHidesOnDeactivate(false);
    }
}
