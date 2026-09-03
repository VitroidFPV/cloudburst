#[cfg(windows)]
pub fn register_development_identity(app: &tauri::AppHandle) -> Result<(), String> {
    let identifier = &app.config().identifier;
    let display_name = app.config().product_name.as_deref().unwrap_or(identifier);
    let icon_uri = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("icons/icon.ico")
        .to_string_lossy()
        .to_string();
    let key = windows_registry::CURRENT_USER
        .create(format!(r"Software\Classes\AppUserModelId\{identifier}"))
        .map_err(|error| format!("Could not register the notification identity: {error}"))?;

    key.set_string("DisplayName", display_name)
        .and_then(|_| key.set_string("IconUri", &icon_uri))
        .and_then(|_| key.set_string("IconBackgroundColor", "0"))
        .map_err(|error| format!("Could not configure the notification identity: {error}"))
}

#[tauri::command]
pub fn send_torrent_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        if crate::build_flavor::is_development(&app) {
            return tauri_winrt_notification::Toast::new(&app.config().identifier)
                .title(&title)
                .text1(&body)
                .show()
                .map_err(|error| format!("Could not show the notification: {error}"));
        }
    }

    {
        use tauri_plugin_notification::NotificationExt;

        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| format!("Could not show the notification: {error}"))
    }
}
