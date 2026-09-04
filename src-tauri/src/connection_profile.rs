use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const STORE_FILE_NAME: &str = "connection-profiles.json";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationMode {
    ApiKey,
    Credentials,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub endpoint: String,
    pub authentication_mode: AuthenticationMode,
    pub username: Option<String>,
}

impl ConnectionProfile {
    /// Profiles are identified by what they describe, so re-adding an
    /// identical connection updates the retained profile instead of
    /// duplicating it.
    pub fn identity(endpoint: &str, mode: AuthenticationMode, username: Option<&str>) -> String {
        format!("{endpoint}|{mode:?}|{}", username.unwrap_or_default())
    }

    pub fn with_computed_id(mut self) -> Self {
        self.id = Self::identity(&self.endpoint, self.authentication_mode, self.username.as_deref());
        self
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfileStore {
    pub active_id: Option<String>,
    pub profiles: Vec<ConnectionProfile>,
}

impl ConnectionProfileStore {
    pub fn active_profile(&self) -> Option<&ConnectionProfile> {
        self.active_id
            .as_deref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == id))
    }

    /// The active profile is attempted first, then the rest in saved order.
    pub fn resolution_order(&self) -> Vec<ConnectionProfile> {
        let mut ordered = Vec::with_capacity(self.profiles.len());
        if let Some(active) = self.active_profile() {
            ordered.push(active.clone());
        }
        ordered.extend(
            self.profiles
                .iter()
                .filter(|profile| Some(profile.id.as_str()) != self.active_id.as_deref())
                .cloned(),
        );
        ordered
    }

    pub fn upsert(&mut self, profile: ConnectionProfile) {
        let profile = profile.with_computed_id();
        match self.profiles.iter_mut().find(|retained| retained.id == profile.id) {
            Some(retained) => *retained = profile.clone(),
            None => self.profiles.push(profile.clone()),
        }
        self.active_id = Some(profile.id);
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let existed = self.profiles.iter().any(|profile| profile.id == id);
        self.profiles.retain(|profile| profile.id != id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
        existed
    }
}

pub async fn load_store(app: &AppHandle) -> Result<ConnectionProfileStore, String> {
    let store_path = store_path(app)?;
    tauri::async_runtime::spawn_blocking(move || load_store_blocking(&store_path))
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

pub async fn persist_store(app: &AppHandle, store: &ConnectionProfileStore) -> Result<(), String> {
    let store_path = store_path(app)?;
    let store = store.clone();
    tauri::async_runtime::spawn_blocking(move || write_store_file(&store_path, &store))
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

pub async fn read_credential(
    app: &AppHandle,
    profile_id: &str,
) -> Result<Option<String>, String> {
    let service = credential_service(app);
    let profile_id = profile_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        read_credential_blocking(&service, &profile_id)
    })
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

pub async fn write_credential(
    app: &AppHandle,
    profile_id: &str,
    secret: String,
) -> Result<(), String> {
    let service = credential_service(app);
    let profile_id = profile_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        write_credential_blocking(&service, &profile_id, &secret)
    })
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

pub async fn delete_credential(app: &AppHandle, profile_id: &str) -> Result<(), String> {
    let service = credential_service(app);
    let profile_id = profile_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        delete_credential_blocking(&service, &profile_id)
    })
        .await
        .map_err(|error| format!("Failed to join the credential-storage task: {error}"))?
}

fn credential_service(app: &AppHandle) -> String {
    credential_service_name(&app.config().identifier)
}

fn credential_service_name(identifier: &str) -> String {
    format!("{identifier}.qbittorrent")
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(STORE_FILE_NAME))
        .map_err(|error| format!("Could not locate Cloudburst's configuration directory: {error}"))
}

fn load_store_blocking(store_path: &PathBuf) -> Result<ConnectionProfileStore, String> {
    read_store_file(store_path).map(|store| store.unwrap_or_default())
}

fn read_store_file(path: &PathBuf) -> Result<Option<ConnectionProfileStore>, String> {
    let json = match fs::read(path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Could not read the saved qBittorrent connection profiles: {error}"
            ));
        }
    };
    serde_json::from_slice::<ConnectionProfileStore>(&json)
        .map(Some)
        .map_err(|error| format!("The saved qBittorrent connection profiles are invalid: {error}"))
}

fn write_store_file(path: &PathBuf, store: &ConnectionProfileStore) -> Result<(), String> {
    let directory = path.parent().ok_or_else(|| {
        "Cloudburst's connection profile path has no parent directory.".to_string()
    })?;
    fs::create_dir_all(directory).map_err(|error| {
        format!("Could not create Cloudburst's configuration directory: {error}")
    })?;

    let json = serde_json::to_vec_pretty(store)
        .map_err(|error| format!("Could not serialize the qBittorrent profiles: {error}"))?;
    let temporary_path = path.with_extension("json.tmp");
    fs::write(&temporary_path, json)
        .map_err(|error| format!("Could not write the qBittorrent profiles: {error}"))?;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Could not replace the qBittorrent profiles: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "Could not finish saving the qBittorrent profiles: {error}"
        ));
    }

    Ok(())
}

fn read_credential_blocking(service: &str, profile_id: &str) -> Result<Option<String>, String> {
    match credential_entry(service, profile_id)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!(
            "Could not read the saved qBittorrent credential: {error}"
        )),
    }
}

fn write_credential_blocking(service: &str, profile_id: &str, secret: &str) -> Result<(), String> {
    credential_entry(service, profile_id)?
        .set_password(secret)
        .map_err(|error| format!("Could not protect the qBittorrent credential: {error}"))
}

fn delete_credential_blocking(service: &str, profile_id: &str) -> Result<(), String> {
    match credential_entry(service, profile_id)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "The profile was removed, but its protected credential could not be deleted: {error}"
        )),
    }
}

fn credential_entry(service: &str, account: &str) -> Result<Entry, String> {
    Entry::new(service, account)
        .map_err(|error| format!("The operating system credential vault is unavailable: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(endpoint: &str, mode: AuthenticationMode, username: Option<&str>) -> ConnectionProfile {
        ConnectionProfile {
            id: String::new(),
            endpoint: endpoint.to_string(),
            authentication_mode: mode,
            username: username.map(str::to_string),
        }
        .with_computed_id()
    }

    #[test]
    fn credential_service_is_scoped_to_the_application_identifier() {
        assert_eq!(
            credential_service_name("dev.vitroid.cloudburst"),
            "dev.vitroid.cloudburst.qbittorrent"
        );
        assert_eq!(
            credential_service_name("dev.vitroid.cloudburst.dev"),
            "dev.vitroid.cloudburst.dev.qbittorrent"
        );
    }

    #[test]
    fn profile_ids_are_deterministic_and_distinguish_profiles() {
        let first = profile("http://localhost:8080", AuthenticationMode::ApiKey, None);
        let duplicate = profile("http://localhost:8080", AuthenticationMode::ApiKey, None);
        let other_host = profile("http://localhost:9090", AuthenticationMode::ApiKey, None);
        let credentials = profile("http://localhost:8080", AuthenticationMode::Credentials, Some("admin"));

        assert_eq!(first.id, duplicate.id);
        assert_ne!(first.id, other_host.id);
        assert_ne!(first.id, credentials.id);
    }

    #[test]
    fn upsert_reuses_the_profile_described_by_the_connection_and_activates_it() {
        let mut store = ConnectionProfileStore::default();
        store.upsert(profile("http://localhost:8080", AuthenticationMode::ApiKey, None));
        store.upsert(profile("http://nas:8080", AuthenticationMode::Credentials, Some("admin")));
        store.upsert(profile("http://localhost:8080", AuthenticationMode::ApiKey, None));

        assert_eq!(store.profiles.len(), 2);
        assert_eq!(
            store.active_id.as_deref(),
            Some(store.profiles[0].id.as_str())
        );
    }

    #[test]
    fn resolution_order_tries_the_active_profile_first_then_the_saved_order() {
        let mut store = ConnectionProfileStore::default();
        store.upsert(profile("http://nas:8080", AuthenticationMode::ApiKey, None));
        store.upsert(profile("http://home:8080", AuthenticationMode::ApiKey, None));
        store.upsert(profile("http://work:8080", AuthenticationMode::ApiKey, None));

        let active_id = store.profiles[1].id.clone();
        store.active_id = Some(active_id.clone());

        let order: Vec<String> = store.resolution_order().into_iter().map(|p| p.id).collect();
        assert_eq!(order, vec![active_id, store.profiles[0].id.clone(), store.profiles[2].id.clone()]);
    }

    #[test]
    fn remove_drops_the_profile_and_clears_an_active_reference() {
        let mut store = ConnectionProfileStore::default();
        store.upsert(profile("http://nas:8080", AuthenticationMode::ApiKey, None));
        let removed_id = store.profiles[0].id.clone();

        assert!(store.remove(&removed_id));
        assert!(!store.remove(&removed_id));
        assert!(store.profiles.is_empty());
        assert_eq!(store.active_id, None);
    }

    #[test]
    fn stores_round_trip_through_json() {
        let mut store = ConnectionProfileStore::default();
        store.upsert(profile("http://localhost:8080", AuthenticationMode::Credentials, Some("admin")));
        let json = serde_json::to_string(&store).unwrap();
        let restored: ConnectionProfileStore = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, store);
    }

    #[test]
    fn serialized_profiles_never_contain_secrets() {
        let mut store = ConnectionProfileStore::default();
        store.upsert(profile("http://localhost:8080", AuthenticationMode::Credentials, Some("admin")));
        let json = serde_json::to_string(&store).unwrap();

        assert!(json.contains("admin"));
        assert!(!json.contains("password"));
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("secret"));
        assert_eq!(
            serde_json::from_str::<ConnectionProfileStore>(&json).unwrap(),
            store
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "writes and immediately removes a dedicated Windows Credential Manager entry"]
    fn windows_credential_vault_round_trip() {
        let service = credential_service_name("dev.vitroid.cloudburst.test");
        let entry = Entry::new(&service, "credential-storage-test").unwrap();
        let test_result = entry
            .set_password("temporary-cloudburst-test-secret")
            .and_then(|()| entry.get_password())
            .map(|secret| assert_eq!(secret, "temporary-cloudburst-test-secret"));
        let cleanup_result = entry.delete_credential();

        test_result.unwrap();
        cleanup_result.unwrap();
    }
}
