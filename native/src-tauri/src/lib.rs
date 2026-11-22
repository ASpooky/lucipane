use tauri::Manager;

/// Cover the primary monitor with the overlay window, make it click-through, then show it.
/// Spike: single display only. Multi-monitor (one window per monitor) comes later.
fn show_overlay(app: &tauri::App) -> tauri::Result<()> {
    let window = app
        .get_webview_window("overlay")
        .expect("overlay window is declared in tauri.conf.json");
    if let Some(monitor) = window.primary_monitor()? {
        window.set_position(*monitor.position())?;
        window.set_size(*monitor.size())?;
    }
    window.set_ignore_cursor_events(true)?;
    window.show()?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            show_overlay(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
