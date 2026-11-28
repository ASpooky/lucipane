//! Following scrolls: when the wheel turns over another app, work out how far that app's
//! content moves, so the overlay can move the marks drawn on it.

use serde::{Deserialize, Serialize};

use crate::pen::Viewport;

/// An area of the overlay in CSS pixels (`Rect` in contract/src/events.ts).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Payload of the "scroll" event (`ScrollEvent` in contract/src/events.ts).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScrollEvent {
    pub window: u64,
    pub rect: Rect,
    /// How far the content moved down, in CSS pixels.
    pub dy: f64,
}

/// The top-level window under the cursor, in global physical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub window: u64,
    /// Executable file name, e.g. `chrome.exe`.
    pub process: String,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// How far an app's content moves per wheel notch, in its logical pixels.
#[derive(Debug, Clone, Copy)]
enum Notch {
    Fixed(f64),
    /// VS Code and its forks: 50 px times `editor.mouseWheelScrollSensitivity`, read from the
    /// user settings in this folder under the config directory.
    VsCode {
        settings_dir: &'static str,
    },
}

/// Measured at 100% display scale with screenshots before and after each notch (the same
/// amount for 1, 3 and 5 notches): Chrome 100 px at any page zoom; VS Code 50 px at any font
/// size, its editor and (rounded to whole rows) its terminal; Windows Terminal 57 px, three
/// rows of its default font. The other Chromium browsers are assumed to match Chrome. Apps
/// not listed are not followed.
const APPS: &[(&str, Notch)] = &[
    ("chrome.exe", Notch::Fixed(100.0)),
    ("msedge.exe", Notch::Fixed(100.0)),
    ("brave.exe", Notch::Fixed(100.0)),
    ("vivaldi.exe", Notch::Fixed(100.0)),
    (
        "code.exe",
        Notch::VsCode {
            settings_dir: "Code",
        },
    ),
    (
        "code - insiders.exe",
        Notch::VsCode {
            settings_dir: "Code - Insiders",
        },
    ),
    (
        "cursor.exe",
        Notch::VsCode {
            settings_dir: "Cursor",
        },
    ),
    (
        "vscodium.exe",
        Notch::VsCode {
            settings_dir: "VSCodium",
        },
    ),
    ("windowsterminal.exe", Notch::Fixed(57.0)),
];

const VSCODE_NOTCH: f64 = 50.0;
const VSCODE_SENSITIVITY: &str = "editor.mouseWheelScrollSensitivity";

/// Pixels per notch for an app, by executable name; `None` for apps we do not follow.
/// `vscode_settings` returns the settings.json in a folder under the config directory.
pub fn notch_px(process: &str, vscode_settings: impl Fn(&str) -> Option<String>) -> Option<f64> {
    let process = process.to_ascii_lowercase();
    let (_, notch) = APPS.iter().find(|(name, _)| *name == process)?;
    Some(match notch {
        Notch::Fixed(px) => *px,
        Notch::VsCode { settings_dir } => {
            VSCODE_NOTCH
                * vscode_settings(settings_dir)
                    .as_deref()
                    .map_or(1.0, vscode_sensitivity)
        }
    })
}

/// `editor.mouseWheelScrollSensitivity` from a VS Code settings.json; 1 when unset or invalid.
pub fn vscode_sensitivity(settings: &str) -> f64 {
    serde_json::from_str::<serde_json::Value>(&strip_jsonc(settings))
        .ok()
        .and_then(|v| v.get(VSCODE_SENSITIVITY)?.as_f64())
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(1.0)
}

/// Turn JSON with comments (VS Code's settings format) into plain JSON: drop `//` and `/* */`
/// comments and trailing commas, leaving string contents alone.
pub fn strip_jsonc(text: &str) -> String {
    without_trailing_commas(&without_comments(text))
}

fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                copy_string_rest(&mut chars, &mut out);
            }
            '/' if chars.peek() == Some(&'/') => while chars.next_if(|&n| n != '\n').is_some() {},
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = '\0';
                for n in chars.by_ref() {
                    if prev == '*' && n == '/' {
                        break;
                    }
                    prev = n;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn without_trailing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                copy_string_rest(&mut chars, &mut out);
            }
            ',' => {
                let next = chars.clone().find(|n| !n.is_whitespace());
                if !matches!(next, Some('}' | ']')) {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Copy the rest of a string literal whose opening quote was just copied.
fn copy_string_rest(chars: &mut std::iter::Peekable<std::str::Chars>, out: &mut String) {
    while let Some(c) = chars.next() {
        out.push(c);
        match c {
            '\\' => out.extend(chars.next()),
            '"' => return,
            _ => {}
        }
    }
}

/// The bars above and below an app's scrolling content (top, bottom), in logical pixels. Marks
/// are clipped to the area between, so ones scrolled under a toolbar disappear. Window handles
/// do not tell where the content is (Chrome draws its pages without a child window), so these
/// were measured: Chrome with its bookmarks bar shown (without it, marks vanish about 30 px
/// early), VS Code's title, tab and breadcrumb bars and status bar, Windows Terminal's tabs.
pub fn insets(process: &str) -> (f64, f64) {
    match process.to_ascii_lowercase().as_str() {
        "chrome.exe" | "msedge.exe" | "brave.exe" | "vivaldi.exe" => (121.0, 0.0),
        "code.exe" | "code - insiders.exe" | "cursor.exe" | "vscodium.exe" => (90.0, 22.0),
        "windowsterminal.exe" => (40.0, 0.0),
        _ => (0.0, 0.0),
    }
}

/// The event for `notches` wheel notches over `target` (positive: away from the user, which
/// scrolls up and moves the content down), for an app scrolling `px` per notch with its
/// content between `insets` (top, bottom).
pub fn event(
    target: &Target,
    notches: f64,
    px: f64,
    (top, bottom): (f64, f64),
    viewport: &Viewport,
) -> ScrollEvent {
    let (x, y) = viewport.to_local(target.left, target.top);
    let height = (target.bottom - target.top) / viewport.scale;
    // Apps scroll and lay out in logical pixels, which are the overlay's CSS pixels.
    ScrollEvent {
        window: target.window,
        rect: Rect {
            x,
            y: y + top,
            width: (target.right - target.left) / viewport.scale,
            height: (height - top - bottom).max(0.0),
        },
        dy: notches * px,
    }
}

/// The event for a wheel turn at a global physical position, if the window there belongs to
/// an app we follow and no modifier turns the wheel into something else (zoom, sideways).
pub fn follow(x: f64, y: f64, notches: f64, viewport: &Viewport) -> Option<ScrollEvent> {
    if platform::modifier_held() {
        return None;
    }
    let target = platform::target_at(x, y)?;
    let px = notch_px(&target.process, read_vscode_settings)?;
    Some(event(
        &target,
        notches,
        px,
        insets(&target.process),
        viewport,
    ))
}

fn read_vscode_settings(dir: &str) -> Option<String> {
    let path = dirs::config_dir()?
        .join(dir)
        .join("User")
        .join("settings.json");
    std::fs::read_to_string(path).ok()
}

#[cfg(windows)]
mod platform {
    use super::Target;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Foundation::{CloseHandle, POINT, RECT};
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GW_HWNDNEXT, GWL_EXSTYLE, GetTopWindow, GetWindow, GetWindowLongW, GetWindowRect,
        GetWindowThreadProcessId, IsWindowVisible, WS_EX_TRANSPARENT,
    };

    /// The front-most top-level window at a point that the wheel would reach. Not
    /// `WindowFromPoint`: it also returns windows a window manager hid by cloaking them (other
    /// GlazeWM workspaces), and click-through ones.
    unsafe fn window_at(point: POINT) -> Option<(HWND, RECT)> {
        // SAFETY (whole body): read-only queries on handles from the window list; a window that
        // closes meanwhile just makes them fail.
        unsafe {
            let mut window = GetTopWindow(std::ptr::null_mut());
            while !window.is_null() {
                let mut rect: RECT = std::mem::zeroed();
                let mut cloaked = 0u32;
                let hidden = IsWindowVisible(window) == 0
                    || DwmGetWindowAttribute(
                        window,
                        DWMWA_CLOAKED as u32,
                        (&raw mut cloaked).cast(),
                        size_of::<u32>() as u32,
                    ) == 0
                        && cloaked != 0;
                let click_through =
                    GetWindowLongW(window, GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT != 0;
                if !hidden
                    && !click_through
                    && GetWindowRect(window, &mut rect) != 0
                    && (rect.left..rect.right).contains(&point.x)
                    && (rect.top..rect.bottom).contains(&point.y)
                {
                    return Some((window, rect));
                }
                window = GetWindow(window, GW_HWNDNEXT);
            }
            None
        }
    }

    /// Ctrl+wheel zooms, Shift+wheel scrolls sideways, Alt+wheel scrolls faster in VS Code.
    pub fn modifier_held() -> bool {
        // SAFETY: GetAsyncKeyState has no preconditions.
        [VK_CONTROL, VK_SHIFT, VK_MENU]
            .into_iter()
            .any(|key| unsafe { GetAsyncKeyState(i32::from(key)) } as u16 & 0x8000 != 0)
    }

    /// The window the wheel turns over, unless it is ours (the overlay in draw mode).
    pub fn target_at(x: f64, y: f64) -> Option<Target> {
        // SAFETY: plain Win32 queries; every handle is checked, and the process handle is
        // closed before returning.
        unsafe {
            let point = POINT {
                x: x as i32,
                y: y as i32,
            };
            let (window, rect) = window_at(point)?;
            // Our own overlay is in front only in draw mode, where it takes the wheel itself.
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            if pid == 0 || pid == std::process::id() {
                return None;
            }
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }
            let mut path = [0u16; 1024];
            let mut len = path.len() as u32;
            let ok = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                path.as_mut_ptr(),
                &mut len,
            );
            CloseHandle(process);
            if ok == 0 {
                return None;
            }
            let path = String::from_utf16_lossy(&path[..len as usize]);
            Some(Target {
                window: window as u64,
                process: path.rsplit('\\').next().unwrap_or_default().to_string(),
                left: f64::from(rect.left),
                top: f64::from(rect.top),
                right: f64::from(rect.right),
                bottom: f64::from(rect.bottom),
            })
        }
    }
}

/// Not followed yet: the macOS mouse hook does not report the wheel.
#[cfg(not(windows))]
mod platform {
    use super::Target;

    pub fn modifier_held() -> bool {
        false
    }

    pub fn target_at(_x: f64, _y: f64) -> Option<Target> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../contract/fixtures/scroll-event.json");

    fn no_settings(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn knows_how_far_measured_apps_scroll_per_notch() {
        assert_eq!(notch_px("chrome.exe", no_settings), Some(100.0));
        assert_eq!(notch_px("Chrome.exe", no_settings), Some(100.0));
        assert_eq!(notch_px("WindowsTerminal.exe", no_settings), Some(57.0));
        assert_eq!(notch_px("Code.exe", no_settings), Some(50.0));
    }

    #[test]
    fn does_not_follow_unknown_apps() {
        assert_eq!(notch_px("notepad.exe", no_settings), None);
    }

    #[test]
    fn scales_vs_code_by_its_scroll_sensitivity() {
        let settings = |dir: &str| {
            (dir == "Code").then(|| r#"{ "editor.mouseWheelScrollSensitivity": 2 }"#.to_string())
        };
        assert_eq!(notch_px("Code.exe", settings), Some(100.0));
        // Cursor keeps its settings elsewhere, so it is not affected.
        assert_eq!(notch_px("Cursor.exe", settings), Some(50.0));
    }

    #[test]
    fn reads_the_sensitivity_from_settings_with_comments() {
        let settings = r#"{
            // speed up "scrolling" // a lot
            "editor.fontSize": 12, /* block
            comment */ "editor.mouseWheelScrollSensitivity": 1.5,
        }"#;
        assert_eq!(vscode_sensitivity(settings), 1.5);
    }

    #[test]
    fn falls_back_to_a_sensitivity_of_one() {
        assert_eq!(vscode_sensitivity("{}"), 1.0);
        assert_eq!(vscode_sensitivity("not json"), 1.0);
        assert_eq!(
            vscode_sensitivity(r#"{ "editor.mouseWheelScrollSensitivity": 0 }"#),
            1.0
        );
        assert_eq!(
            vscode_sensitivity(r#"{ "editor.mouseWheelScrollSensitivity": "2" }"#),
            1.0
        );
    }

    #[test]
    fn strips_comments_and_trailing_commas_but_not_string_contents() {
        assert_eq!(
            strip_jsonc(
                r#"{"a": "http://x/*y*/", /* c */ "b": [1, 2,], // d
}"#
            ),
            "{\"a\": \"http://x/*y*/\",  \"b\": [1, 2] \n}"
        );
        assert_eq!(
            strip_jsonc(r#"{"q": "say \"//\""}"#),
            r#"{"q": "say \"//\""}"#
        );
    }

    #[test]
    fn converts_the_window_to_overlay_pixels_and_notches_to_content_movement() {
        let viewport = Viewport {
            origin_x: 0.0,
            origin_y: 0.0,
            scale: 1.5,
        };
        let target = Target {
            window: 42,
            process: "chrome.exe".into(),
            left: 150.0,
            top: 300.0,
            right: 1650.0,
            bottom: 1200.0,
        };
        // One notch towards the user scrolls down: the content moves up.
        assert_eq!(
            event(&target, -1.0, 100.0, (0.0, 0.0), &viewport),
            ScrollEvent {
                window: 42,
                rect: Rect {
                    x: 100.0,
                    y: 200.0,
                    width: 1000.0,
                    height: 600.0
                },
                dy: -100.0,
            }
        );
        assert_eq!(event(&target, 3.0, 57.0, (0.0, 0.0), &viewport).dy, 171.0);
        // Toolbars and status bars (in CSS pixels) are left out of the area.
        let rect = event(&target, 1.0, 100.0, (121.0, 22.0), &viewport).rect;
        assert_eq!((rect.y, rect.height), (321.0, 457.0));
    }

    #[test]
    fn leaves_out_the_bars_around_an_app_s_content() {
        assert_eq!(insets("chrome.exe"), (121.0, 0.0));
        assert_eq!(insets("Code.exe"), (90.0, 22.0));
        assert_eq!(insets("WindowsTerminal.exe"), (40.0, 0.0));
        assert_eq!(insets("notepad.exe"), (0.0, 0.0));
    }

    #[test]
    fn serializes_as_the_shared_contract_fixture() {
        let event = ScrollEvent {
            window: 263174,
            rect: Rect {
                x: 13.0,
                y: 60.0,
                width: 1894.0,
                height: 959.0,
            },
            dy: -300.0,
        };
        let fixture: ScrollEvent = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(fixture, event);
        let json = serde_json::to_value(event).unwrap();
        let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["dy", "rect", "window"]);
    }
}
