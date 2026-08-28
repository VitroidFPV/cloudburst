mod connection_profile;
mod qbittorrent;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(qbittorrent::ConnectionManager::default())
        .setup(tray::setup)
        .on_window_event(tray::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            qbittorrent::connect_qbittorrent,
            qbittorrent::restore_saved_qbittorrent,
            qbittorrent::refresh_qbittorrent,
            qbittorrent::set_torrents_paused,
            qbittorrent::disconnect_qbittorrent
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
