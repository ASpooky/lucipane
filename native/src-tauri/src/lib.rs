mod focus;
mod pen;

use std::sync::Mutex;

use pen::{Input, PenEvent, PenState, Viewport};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, ipc::Channel};

const OVERLAY: &str = "overlay";

/// The WebView's channel for pen events, set once the page subscribes.
#[derive(Default)]
struct PenSink(Mutex<Option<Channel<PenEvent>>>);

/// Draw mode: the overlay takes the mouse and keyboard so the user can draw with the left
/// button and use single-key commands. Outside it, the overlay is click-through.
#[derive(Default)]
struct DrawMode(Mutex<DrawModeState>);

#[derive(Default)]
struct DrawModeState {
    enabled: bool,
    /// The window to give focus back to when draw mode ends.
    previous_focus: Option<focus::WindowHandle>,
}

fn overlay(app: &AppHandle) -> WebviewWindow {
    app.get_webview_window(OVERLAY)
        .expect("overlay window is declared in tauri.conf.json")
}

fn apply_draw_mode(app: &AppHandle, enabled: bool) -> tauri::Result<()> {
    // Update the state under the lock, but touch the window only after releasing it:
    // window calls hop to the main thread, which may itself be waiting for this lock.
    let previous_focus = {
        let state = app.state::<DrawMode>();
        let mut mode = state.0.lock().expect("draw mode poisoned");
        if mode.enabled == enabled {
            return Ok(());
        }
        mode.enabled = enabled;
        if enabled {
            mode.previous_focus = focus::foreground();
            None
        } else {
            mode.previous_focus.take()
        }
    };
    let window = overlay(app);
    if enabled {
        window.set_ignore_cursor_events(false)?;
        window.set_focus()?;
    } else {
        window.set_ignore_cursor_events(true)?;
        if let Some(previous) = previous_focus {
            focus::restore(previous);
        }
    }
    app.emit("draw-mode", enabled)
}

fn toggle_draw_mode(app: &AppHandle) {
    let enabled = app
        .state::<DrawMode>()
        .0
        .lock()
        .expect("draw mode poisoned")
        .enabled;
    if let Err(e) = apply_draw_mode(app, !enabled) {
        eprintln!("failed to toggle draw mode: {e}");
    }
}

#[tauri::command]
fn subscribe_pen(sink: State<'_, PenSink>, on_event: Channel<PenEvent>) {
    *sink.0.lock().expect("pen sink poisoned") = Some(on_event);
}

#[tauri::command]
fn set_draw_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    apply_draw_mode(&app, enabled).map_err(|e| e.to_string())
}

/// Cover the primary monitor with the overlay window, make it click-through, then show it.
/// Spike: single display only. Multi-monitor (one window per monitor) comes later.
fn show_overlay(app: &AppHandle) -> tauri::Result<Viewport> {
    let window = overlay(app);
    let mut viewport = Viewport {
        origin_x: 0.0,
        origin_y: 0.0,
        scale: 1.0,
    };
    if let Some(monitor) = window.primary_monitor()? {
        window.set_position(*monitor.position())?;
        window.set_size(*monitor.size())?;
        viewport = Viewport {
            origin_x: f64::from(monitor.position().x),
            origin_y: f64::from(monitor.position().y),
            scale: monitor.scale_factor(),
        };
    }
    window.set_ignore_cursor_events(true)?;
    window.show()?;
    Ok(viewport)
}

/// Register the global hotkey that toggles draw mode (Ctrl+Alt+Shift+D).
/// Registration fails if another app already owns the key; we log it and keep running.
#[cfg(desktop)]
fn register_draw_mode_hotkey(app: &AppHandle) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };
    let hotkey = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::KeyD,
    );
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, shortcut, event| {
                if shortcut == &hotkey && event.state() == ShortcutState::Pressed {
                    toggle_draw_mode(app);
                }
            })
            .build(),
    )?;
    if let Err(e) = app.global_shortcut().register(hotkey) {
        eprintln!("could not register the draw mode hotkey (in use by another app?): {e}");
    }
    Ok(())
}

fn to_input(event: &rdev::EventType) -> Option<Input> {
    match *event {
        rdev::EventType::MouseMove { x, y } => Some(Input::Move { x, y }),
        rdev::EventType::ButtonPress(rdev::Button::Middle) => Some(Input::MiddleDown),
        rdev::EventType::ButtonRelease(rdev::Button::Middle) => Some(Input::MiddleUp),
        _ => None,
    }
}

/// Listen to global mouse input on a dedicated thread (`rdev::listen` blocks forever)
/// and forward pen events to the subscribed WebView.
fn spawn_mouse_hook(app: AppHandle, viewport: Viewport) {
    std::thread::spawn(move || {
        let mut state = PenState::default();
        let result = rdev::listen(move |event| {
            let Some(input) = to_input(&event.event_type) else {
                return;
            };
            let (next, pen_event) = pen::step(state, input, &viewport);
            state = next;
            let Some(pen_event) = pen_event else { return };
            let sink = app.state::<PenSink>();
            if let Some(channel) = sink.0.lock().expect("pen sink poisoned").as_ref() {
                let _ = channel.send(pen_event);
            }
        });
        if let Err(e) = result {
            eprintln!("global mouse hook failed: {e:?}");
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(PenSink::default())
        .manage(DrawMode::default())
        .invoke_handler(tauri::generate_handler![subscribe_pen, set_draw_mode])
        .setup(|app| {
            let handle = app.handle();
            let viewport = show_overlay(handle)?;
            #[cfg(desktop)]
            register_draw_mode_hotkey(handle)?;
            spawn_mouse_hook(handle.clone(), viewport);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
