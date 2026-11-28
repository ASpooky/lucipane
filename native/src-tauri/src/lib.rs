mod annotate;
mod board;
pub mod cli;
mod focus;
mod mouse_hook;
#[cfg(target_os = "macos")]
mod panel;
mod pen;
mod scroll;
pub mod shape;
mod workspace;

use std::sync::{Mutex, mpsc};

use board::BoardSnapshot;
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
fn current_workspace() -> String {
    workspace::current()
}

#[tauri::command]
fn subscribe_pen(sink: State<'_, PenSink>, on_event: Channel<PenEvent>) {
    *sink.0.lock().expect("pen sink poisoned") = Some(on_event);
}

#[tauri::command]
fn set_draw_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    apply_draw_mode(&app, enabled).map_err(|e| e.to_string())
}

/// Persist the finished strokes for the `lucipane capture` CLI.
#[tauri::command]
fn save_board(snapshot: BoardSnapshot) -> Result<(), String> {
    if snapshot.contract_version != board::CONTRACT_VERSION {
        return Err(format!(
            "unexpected contract version {}",
            snapshot.contract_version
        ));
    }
    write_board(&snapshot)
}

fn write_board(snapshot: &BoardSnapshot) -> Result<(), String> {
    let path = board::board_file().ok_or("no data directory on this system")?;
    let json = serde_json::to_string(snapshot).map_err(|e| e.to_string())?;
    board::write_atomically(&path, &json).map_err(|e| format!("{}: {e}", path.display()))
}

/// The overlay starts empty, so the board file must too: a file left over from an earlier
/// run would otherwise report strokes that are no longer on screen.
fn reset_board(scale: f64) {
    if let Err(e) = write_board(&BoardSnapshot::empty(scale)) {
        eprintln!("could not reset the board file: {e}");
    }
}

fn remove_board() {
    if let Some(path) = board::board_file() {
        let _ = std::fs::remove_file(path);
    }
}

/// Another `lucipane ...` was started while this overlay is running; the single-instance
/// plugin hands us its arguments and exits it.
fn on_second_instance(app: &AppHandle, argv: &[String]) {
    // argv[0] is the program path. Capture and help run in the other process and never get
    // here, and a plain second launch has nothing to do.
    let result = match cli::parse(argv.get(1..).unwrap_or_default()) {
        Ok(Some(cli::Command::Clear {
            agent_only,
            all_desktops,
        })) => app.emit(
            "clear",
            ClearRequest {
                agent_only,
                all_desktops,
            },
        ),
        Ok(Some(cli::Command::Draw(shape))) => {
            // A path can be several strokes; the WebView adds each "agent-draw" event as one.
            let scale = app.state::<OverlayScale>().0;
            shape::to_strokes(&shape, scale)
                .into_iter()
                .try_for_each(|stroke| app.emit("agent-draw", stroke))
        }
        _ => Ok(()),
    };
    if let Err(e) = result {
        eprintln!("failed to forward a command to the overlay: {e}");
    }
}

/// Payload of the "clear" event (see `listenToRust` in main.ts).
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearRequest {
    /// Erase only the agent's strokes.
    agent_only: bool,
    /// Clear the panes of every virtual desktop, not just the current one.
    all_desktops: bool,
}

/// Image (physical) pixels per CSS pixel on the overlay's display, for converting the
/// coordinates of `lucipane draw`.
struct OverlayScale(f64);

/// Cover the primary monitor with the overlay window, make it click-through, then show it.
/// Spike: single display only. Multi-monitor (one window per monitor) comes later.
fn show_overlay(app: &AppHandle) -> tauri::Result<Viewport> {
    let window = overlay(app);
    let mut viewport = Viewport {
        origin_x: 0.0,
        origin_y: 0.0,
        scale: 1.0,
    };
    #[cfg(target_os = "macos")]
    float_over_everything(&window)?;
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

/// One pane over everything, like on Windows: macOS would otherwise move the window below the
/// menu bar (shifting every stroke by its height) and leave it out of the Space that each
/// full-screen window gets.
#[cfg(target_os = "macos")]
fn float_over_everything(window: &WebviewWindow) -> tauri::Result<()> {
    use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};
    let ns_window = window.ns_window()?.cast::<NSWindow>();
    // SAFETY: Tauri returns a live NSWindow, and setup runs on the main thread.
    unsafe {
        let ns_window = &*ns_window;
        panel::make_panel(ns_window);
        ns_window.setLevel(NSStatusWindowLevel);
        ns_window.setCollectionBehavior(
            ns_window.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
    }
    Ok(())
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

/// Listen to global mouse input on a dedicated thread and forward pen events to the
/// subscribed WebView. Wheel turns go to a second thread that works out what scrolled.
fn spawn_mouse_hook(app: AppHandle, viewport: Viewport) {
    let wheel = spawn_scroll_follower(app.clone(), viewport);
    std::thread::spawn(move || {
        let mut state = PenState::default();
        let result = mouse_hook::listen(viewport.scale, move |input| {
            if let (Input::Wheel { notches }, Some((x, y))) = (input, state.cursor) {
                let _ = wheel.send((x, y, notches));
            }
            let (next, pen_event) = pen::step(state, input, &viewport);
            state = next;
            let Some(pen_event) = pen_event else { return };
            let sink = app.state::<PenSink>();
            if let Some(channel) = sink.0.lock().expect("pen sink poisoned").as_ref() {
                let _ = channel.send(pen_event);
            }
        });
        if let Err(e) = result {
            eprintln!("global mouse hook failed: {e}");
        }
    });
}

/// Turn wheel notches at a cursor position into "scroll" events for the WebView. Looking up
/// the window, its app and its page takes system calls, a file read and waiting for scrolling
/// to settle, none of which may happen inside the mouse hook: Windows drops a slow hook.
fn spawn_scroll_follower(app: AppHandle, viewport: Viewport) -> mpsc::Sender<(f64, f64, f64)> {
    let (sender, receiver) = mpsc::channel::<(f64, f64, f64)>();
    std::thread::spawn(move || {
        scroll::follow_wheel(receiver, viewport, |event| {
            let _ = app.emit("scroll", event);
        });
    });
    sender
}

/// Start the overlay. With `forwarded = Some(..)`, this launch only exists to hand a command
/// to an overlay that is already running; if there is none, report that and exit.
pub fn run(forwarded: Option<cli::Command>) {
    let mut builder = tauri::Builder::default();
    // Must be the first plugin: a second instance exits inside it, before anything else runs.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            on_second_instance(app, &argv)
        }));
    }
    builder
        .plugin(tauri_plugin_opener::init())
        .manage(PenSink::default())
        .manage(DrawMode::default())
        .invoke_handler(tauri::generate_handler![
            subscribe_pen,
            current_workspace,
            set_draw_mode,
            save_board
        ])
        .setup(move |app| {
            if let Some(command) = forwarded {
                // We are the only instance, so there was nobody to forward to.
                std::process::exit(cli::run(command));
            }
            // Only an accessory app (no Dock icon, no menu bar) may show its windows over
            // another app's full-screen Space.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle();
            let viewport = show_overlay(handle)?;
            reset_board(viewport.scale);
            app.manage(OverlayScale(viewport.scale));
            #[cfg(desktop)]
            register_draw_mode_hotkey(handle)?;
            spawn_mouse_hook(handle.clone(), viewport);
            let events = handle.clone();
            workspace::watch(move |id| {
                let _ = events.emit("workspace", id);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                remove_board();
            }
        });
}
