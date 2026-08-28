use serde::Serialize;

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MagnetHandlerStatus {
    CloudburstDefault,
    OtherProgram,
    NotRegistered,
}

#[tauri::command]
pub async fn magnet_handler_status(app: tauri::AppHandle) -> Result<MagnetHandlerStatus, String> {
    detect_magnet_handler_status(&app)
}

#[tauri::command]
pub async fn open_default_apps_settings() -> Result<(), String> {
    open_default_apps_settings_impl()
}

#[cfg(windows)]
fn detect_magnet_handler_status(app: &tauri::AppHandle) -> Result<MagnetHandlerStatus, String> {
    use tauri_plugin_deep_link::DeepLinkExt;

    let registered = app.deep_link().is_registered("magnet").unwrap_or(false);
    let user_choice = current_user_choice_progid();
    let handler_command = user_choice.as_deref().and_then(resolve_progid_command);
    let marker = current_exe_marker();

    Ok(evaluate_magnet_status(
        registered,
        user_choice,
        handler_command,
        &marker,
    ))
}

#[cfg(not(windows))]
fn detect_magnet_handler_status(_app: &tauri::AppHandle) -> Result<MagnetHandlerStatus, String> {
    // Only Windows routes protocol clicks through a replaceable per-user choice.
    Ok(MagnetHandlerStatus::CloudburstDefault)
}

// Windows consults this protected per-user choice before any Classes
// registration, so a handler chosen in Settings always wins.
#[cfg(windows)]
const USER_CHOICE_PATH: &str = r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\magnet\UserChoice";

#[cfg(windows)]
fn current_user_choice_progid() -> Option<String> {
    windows_registry::CURRENT_USER
        .open(USER_CHOICE_PATH)
        .ok()
        .and_then(|key| key.get_string("ProgId").ok())
}

#[cfg(windows)]
fn resolve_progid_command(progid: &str) -> Option<String> {
    windows_registry::CLASSES_ROOT
        .open(format!(r"{progid}\shell\open\command"))
        .ok()
        .and_then(|key| key.get_string("").ok())
}

fn current_exe_marker() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
        })
        .unwrap_or_else(|| "cloudburst.exe".to_string())
}

fn evaluate_magnet_status(
    registered: bool,
    user_choice: Option<String>,
    handler_command: Option<String>,
    exe_marker: &str,
) -> MagnetHandlerStatus {
    if let Some(progid) = user_choice {
        let progid_is_ours = progid.to_ascii_lowercase().contains(exe_marker);
        let command_is_ours = handler_command
            .map(|command| command.to_ascii_lowercase().contains(exe_marker))
            .unwrap_or(false);

        return if progid_is_ours || command_is_ours {
            MagnetHandlerStatus::CloudburstDefault
        } else {
            MagnetHandlerStatus::OtherProgram
        };
    }

    if registered {
        MagnetHandlerStatus::CloudburstDefault
    } else {
        MagnetHandlerStatus::NotRegistered
    }
}

#[cfg(windows)]
fn open_default_apps_settings_impl() -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg("ms-settings:defaultapps")
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open Windows Settings: {error}"))
}

#[cfg(not(windows))]
fn open_default_apps_settings_impl() -> Result<(), String> {
    Err("Default app settings are only available on Windows.".to_string())
}

// A bare `magnet\shell\open\command` handles direct activation but is
// invisible to handler enumeration, so browsers and the Settings app
// cannot offer Cloudburst as a choice. The capability registration below
// (a ProgId, an OpenWithProgids entry, and a Capabilities block advertised
// through RegisteredApplications) makes Cloudburst enumerable everywhere
// Windows asks "which apps can handle magnet links?".
#[cfg(windows)]
const MAGNET_PROGID: &str = "Cloudburst.Magnet";

#[cfg(windows)]
fn registration_plan(exe_path: &str) -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    let command = format!("\"{exe_path}\" \"%1\"");
    vec![
        (
            r"Software\Classes\Cloudburst.Magnet",
            vec![
                ("", "Cloudburst Magnet Link".to_string()),
                ("URL Protocol", String::new()),
            ],
        ),
        (
            r"Software\Classes\Cloudburst.Magnet\shell\open\command",
            vec![("", command.clone())],
        ),
        (
            r"Software\Classes\magnet\OpenWithProgids",
            vec![(MAGNET_PROGID, String::new())],
        ),
        (
            r"Software\Cloudburst\Capabilities",
            vec![
                ("ApplicationName", "Cloudburst".to_string()),
                (
                    "ApplicationDescription",
                    "A focused desktop interface for qBittorrent".to_string(),
                ),
            ],
        ),
        (
            r"Software\Cloudburst\Capabilities\URLAssociations",
            vec![("magnet", MAGNET_PROGID.to_string())],
        ),
        (
            "Software\\RegisteredApplications",
            vec![("Cloudburst", r"Software\Cloudburst\Capabilities".to_string())],
        ),
    ]
}

#[cfg(windows)]
pub fn register_capability_keys() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("Could not locate the Cloudburst executable: {error}"))?
        .to_string_lossy()
        .to_string();

    for (path, values) in registration_plan(&exe_path) {
        let key = windows_registry::CURRENT_USER
            .create(path)
            .map_err(|error| format!("Could not create the registry key {path}: {error}"))?;
        for (name, value) in values {
            key.set_string(name, &value)
                .map_err(|error| format!("Could not write the registry value {name}: {error}"))?;
        }
    }

    Ok(())
}

#[cfg(windows)]
#[test]
fn writes_the_capability_registration() {
    let exe = r"C:\Program Files\Cloudburst\cloudburst.exe";

    let plan = registration_plan(exe);
    let command = format!("\"{exe}\" \"%1\"");

    assert_eq!(plan.len(), 6);
    assert!(plan.iter().any(|(path, values)| {
        *path == r"Software\Classes\Cloudburst.Magnet\shell\open\command"
            && values.iter().any(|(name, value)| *name == "" && value == &command)
    }));
    assert!(plan.iter().any(|(path, values)| {
        *path == r"Software\Classes\magnet\OpenWithProgids"
            && values.iter().any(|(name, _)| *name == MAGNET_PROGID)
    }));
    assert!(plan.iter().any(|(path, values)| {
        *path == "Software\\RegisteredApplications"
            && values.iter().any(|(name, value)| *name == "Cloudburst" && value == r"Software\Cloudburst\Capabilities")
    }));

    // Exercises the registry round trip with this build's real exe path.
    register_capability_keys().unwrap();
    let command_key = windows_registry::CURRENT_USER
        .open(r"Software\Classes\Cloudburst.Magnet\shell\open\command")
        .unwrap();
    let command_value = command_key.get_string("").unwrap();
    assert!(command_value.contains("cloudburst"));
    assert!(command_value.ends_with("\" \"%1\""));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_user_choice_outweighs_the_registry_registration() {
        assert_eq!(
            evaluate_magnet_status(
                true,
                Some("qBittorrent".to_string()),
                Some(r#""C:\Program Files\qBittorrent\qbittorrent.exe" "%1""#.to_string()),
                "cloudburst.exe",
            ),
            MagnetHandlerStatus::OtherProgram
        );
        assert_eq!(
            evaluate_magnet_status(
                true,
                Some(r"Applications\cloudburst.exe".to_string()),
                None,
                "cloudburst.exe",
            ),
            MagnetHandlerStatus::CloudburstDefault
        );
    }

    #[test]
    fn falls_back_to_the_registry_registration_without_a_user_choice() {
        assert_eq!(
            evaluate_magnet_status(true, None, None, "cloudburst.exe"),
            MagnetHandlerStatus::CloudburstDefault
        );
        assert_eq!(
            evaluate_magnet_status(false, None, None, "cloudburst.exe"),
            MagnetHandlerStatus::NotRegistered
        );
    }
}
