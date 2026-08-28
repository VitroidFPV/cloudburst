use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const CREDENTIAL_SERVICE: &str = "dev.vitroid.cloudburst.qbittorrent";
const CREDENTIAL_ACCOUNT: &str = "active-connection";
const PROFILE_FILE_NAME: &str = "connection-profile.json";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationMode {
    ApiKey,
    Credentials,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub endpoint: String,
    pub authentication_mode: AuthenticationMode,
    pub username: Option<String>,
}

pub struct StoredConnection {
    pub profile: ConnectionProfile,
    pub secret: String,
}

pub async fn load(app: &AppHandle) -> Result<Option<StoredConnection>, String> {
    let profile_path = profile_path(app)?;
    tauri::async_runtime::spawn_blocking(move || load_blocking(profile_path))
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

pub async fn save(
    app: &AppHandle,
    profile: ConnectionProfile,
    secret: String,
) -> Result<(), String> {
    let profile_path = profile_path(app)?;
    tauri::async_runtime::spawn_blocking(move || save_blocking(profile_path, profile, secret))
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

pub async fn clear(app: &AppHandle) -> Result<(), String> {
    let profile_path = profile_path(app)?;
    tauri::async_runtime::spawn_blocking(move || clear_blocking(profile_path))
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

fn profile_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(PROFILE_FILE_NAME))
        .map_err(|error| format!("Could not locate Cloudburst's configuration directory: {error}"))
}

fn load_blocking(profile_path: PathBuf) -> Result<Option<StoredConnection>, String> {
    let profile_json = match fs::read(&profile_path) {
        Ok(profile) => profile,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Could not read the saved qBittorrent connection profile: {error}"
            ));
        }
    };
    let profile = serde_json::from_slice::<ConnectionProfile>(&profile_json)
        .map_err(|error| format!("The saved qBittorrent connection profile is invalid: {error}"))?;
    let secret = credential_entry()?
        .get_password()
        .map_err(|error| match error {
            KeyringError::NoEntry => {
                "The saved qBittorrent profile has no credential in the operating system vault."
                    .to_string()
            }
            error => format!("Could not read the saved qBittorrent credential: {error}"),
        })?;

    Ok(Some(StoredConnection { profile, secret }))
}

fn save_blocking(
    profile_path: PathBuf,
    profile: ConnectionProfile,
    secret: String,
) -> Result<(), String> {
    let entry = credential_entry()?;
    let previous_secret = match entry.get_password() {
        Ok(secret) => Some(secret),
        Err(KeyringError::NoEntry) => None,
        Err(error) => {
            return Err(format!(
                "Could not inspect the existing qBittorrent credential: {error}"
            ));
        }
    };

    entry
        .set_password(&secret)
        .map_err(|error| format!("Could not protect the qBittorrent credential: {error}"))?;

    if let Err(error) = write_profile(&profile_path, &profile) {
        if let Some(previous_secret) = previous_secret {
            let _ = entry.set_password(&previous_secret);
        } else {
            let _ = entry.delete_credential();
        }
        return Err(error);
    }

    Ok(())
}

fn clear_blocking(profile_path: PathBuf) -> Result<(), String> {
    match fs::remove_file(&profile_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Could not remove the saved qBittorrent connection profile: {error}"
            ));
        }
    }

    match credential_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "The profile was removed, but its protected credential could not be deleted: {error}"
        )),
    }
}

fn write_profile(path: &PathBuf, profile: &ConnectionProfile) -> Result<(), String> {
    let directory = path.parent().ok_or_else(|| {
        "Cloudburst's connection profile path has no parent directory.".to_string()
    })?;
    fs::create_dir_all(directory).map_err(|error| {
        format!("Could not create Cloudburst's configuration directory: {error}")
    })?;

    let json = serde_json::to_vec_pretty(profile)
        .map_err(|error| format!("Could not serialize the qBittorrent profile: {error}"))?;
    let temporary_path = path.with_extension("json.tmp");
    fs::write(&temporary_path, json)
        .map_err(|error| format!("Could not write the qBittorrent profile: {error}"))?;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Could not replace the qBittorrent profile: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "Could not finish saving the qBittorrent profile: {error}"
        ));
    }

    Ok(())
}

fn credential_entry() -> Result<Entry, String> {
    Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT)
        .map_err(|error| format!("The operating system credential vault is unavailable: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_profiles_never_contain_secrets() {
        let profile = ConnectionProfile {
            endpoint: "http://localhost:8080".to_string(),
            authentication_mode: AuthenticationMode::Credentials,
            username: Some("admin".to_string()),
        };
        let json = serde_json::to_string(&profile).unwrap();

        assert!(json.contains("admin"));
        assert!(!json.contains("password"));
        assert!(!json.contains("apiKey"));
        assert_eq!(
            serde_json::from_str::<ConnectionProfile>(&json).unwrap(),
            profile
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "writes and immediately removes a dedicated Windows Credential Manager entry"]
    fn windows_credential_vault_round_trip() {
        let entry = Entry::new(CREDENTIAL_SERVICE, "credential-storage-test").unwrap();
        let test_result = entry
            .set_password("temporary-cloudburst-test-secret")
            .and_then(|()| entry.get_password())
            .map(|secret| assert_eq!(secret, "temporary-cloudburst-test-secret"));
        let cleanup_result = entry.delete_credential();

        test_result.unwrap();
        cleanup_result.unwrap();
    }
}
