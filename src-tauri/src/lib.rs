mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(tray::setup)
        .on_window_event(tray::handle_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
