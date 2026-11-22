mod pen;

use std::sync::Mutex;

use pen::{Input, PenEvent, PenState, Viewport};
use tauri::{AppHandle, Manager, State, ipc::Channel};

/// The WebView's channel for pen events, set once the page subscribes.
#[derive(Default)]
struct PenSink(Mutex<Option<Channel<PenEvent>>>);

#[tauri::command]
fn subscribe_pen(sink: State<'_, PenSink>, on_event: Channel<PenEvent>) {
    *sink.0.lock().expect("pen sink poisoned") = Some(on_event);
}

/// Cover the primary monitor with the overlay window, make it click-through, then show it.
/// Spike: single display only. Multi-monitor (one window per monitor) comes later.
fn show_overlay(app: &tauri::App) -> tauri::Result<Viewport> {
    let window = app
        .get_webview_window("overlay")
        .expect("overlay window is declared in tauri.conf.json");
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
        .invoke_handler(tauri::generate_handler![subscribe_pen])
        .setup(|app| {
            let viewport = show_overlay(app)?;
            spawn_mouse_hook(app.handle().clone(), viewport);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
