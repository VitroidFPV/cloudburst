mod connection_profile;
mod magnet_handler;
mod qbittorrent;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // The deep-link feature on this plugin forwards the new instance's
            // URL to `deep_link().on_open_url()` listeners before this runs.
            tray::restore_window_main(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(qbittorrent::ConnectionManager::default())
        .setup(|app| {
            tray::setup(app)?;
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Dev builds and Linux AppImages are not touched by the
                // installer, so claim the configured schemes for this exe.
                let _ = app.deep_link().register_all();
            }
            #[cfg(windows)]
            {
                // Make Cloudburst enumerable as a magnet handler for
                // browsers and the Settings app, not just directly activatable.
                if let Err(error) = magnet_handler::register_capability_keys() {
                    eprintln!("failed to register Cloudburst's magnet capabilities: {error}");
                }
            }
            Ok(())
        })
        .on_window_event(tray::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            qbittorrent::connect_qbittorrent,
            qbittorrent::resolve_connection,
            qbittorrent::connect_saved_qbittorrent,
            qbittorrent::remove_connection_profile,
            qbittorrent::list_connection_profiles,
            qbittorrent::refresh_qbittorrent,
            qbittorrent::set_torrents_paused,
            qbittorrent::remove_torrents,
            qbittorrent::add_torrents,
            qbittorrent::fetch_default_save_path,
            qbittorrent::parse_torrent_metadata,
            qbittorrent::fetch_torrent_metadata,
            qbittorrent::fetch_torrent_properties,
            qbittorrent::fetch_torrent_files,
            qbittorrent::fetch_torrent_trackers,
            qbittorrent::set_torrent_file_priorities,
            qbittorrent::set_torrent_category,
            qbittorrent::add_torrent_tags,
            qbittorrent::remove_torrent_tags,
            qbittorrent::fetch_categories,
            qbittorrent::fetch_tags,
            qbittorrent::disconnect_qbittorrent,
            magnet_handler::magnet_handler_status,
            magnet_handler::open_default_apps_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
