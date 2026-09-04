mod build_flavor;
mod connection_profile;
mod content_action;
mod magnet_handler;
mod notification;
mod qbittorrent;
mod tray;
mod window_appearance;

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
        .plugin(tauri_plugin_notification::init())
        .manage(qbittorrent::ConnectionManager::default())
        .setup(|app| {
            tray::setup(app)?;
            #[cfg(target_os = "linux")]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Linux AppImages are not touched by an installer. Development
                // stays opt-in so it cannot replace production as the handler.
                if !build_flavor::is_development(app.handle()) {
                    let _ = app.deep_link().register_all();
                }
            }
            #[cfg(windows)]
            {
                // Make each flavor enumerable as a separate magnet handler for
                // browsers and the Settings app, not just directly activatable.
                if let Err(error) = magnet_handler::register_capability_keys(app.handle()) {
                    eprintln!("failed to register Cloudburst's magnet capabilities: {error}");
                }
            }
            #[cfg(windows)]
            if build_flavor::is_development(app.handle()) {
                if let Err(error) = notification::register_development_identity(app.handle()) {
                    eprintln!("failed to register Cloudburst's notification identity: {error}");
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
            qbittorrent::perform_torrent_content_action,
            qbittorrent::set_torrent_file_priorities,
            qbittorrent::set_torrent_category,
            qbittorrent::add_torrent_tags,
            qbittorrent::remove_torrent_tags,
            qbittorrent::fetch_categories,
            qbittorrent::fetch_tags,
            qbittorrent::disconnect_qbittorrent,
            magnet_handler::magnet_handler_status,
            magnet_handler::open_default_apps_settings,
            notification::send_torrent_notification,
            window_appearance::set_window_caption_color
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
