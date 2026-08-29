use crate::connection_profile::{self, AuthenticationMode, ConnectionProfile, StoredConnection};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use reqwest::{
    multipart::{Form, Part},
    Client, RequestBuilder, StatusCode, Url,
};
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};
use tauri::State;
use tokio::sync::Mutex;

const MINIMUM_VERSION: (u64, u64, u64) = (5, 2, 0);

#[derive(Default)]
pub struct ConnectionManager {
    // Held for the full duration of each command so connection intents complete in order.
    active: Mutex<Option<ActiveConnection>>,
}

#[derive(Clone)]
struct ActiveConnection {
    client: QbittorrentClient,
    version: String,
}

#[derive(Clone)]
struct QbittorrentClient {
    http: Client,
    endpoint: Url,
    authentication: Authentication,
}

#[derive(Clone)]
enum Authentication {
    ApiKey(String),
    Credentials { username: String, password: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    endpoint: String,
    authentication_mode: AuthenticationMode,
    api_key: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSnapshot {
    endpoint: String,
    version: String,
    torrents: Vec<Torrent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    profile: Option<ConnectionProfile>,
    snapshot: Option<ConnectionSnapshot>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTorrentsInput {
    urls: Vec<String>,
    #[serde(default)]
    files: Vec<AddTorrentFile>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    save_path: Option<String>,
    content_layout: AddContentLayout,
    #[serde(default)]
    file_priorities: Option<Vec<u32>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTorrentFile {
    name: String,
    base64_content: String,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AddContentLayout {
    Original,
    Subfolder,
    NoSubfolder,
}

impl AddContentLayout {
    fn as_qbittorrent_value(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Subfolder => "Subfolder",
            Self::NoSubfolder => "NoSubfolder",
        }
    }
}

#[derive(Serialize, Clone, Default, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddTorrentsOutcome {
    success_count: u32,
    failure_count: u32,
    pending_count: u32,
    added_torrent_ids: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentMetadataFile {
    path: String,
    length: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentMetadata {
    hash: String,
    name: String,
    files: Vec<TorrentMetadataFile>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum MetadataFetch {
    Ready { metadata: TorrentMetadata },
    Pending,
}

/// qBittorrent accepts exactly these per-file priorities; other values are
/// rejected with HTTP 400.
pub const TORRENT_FILE_PRIORITIES: [u32; 4] = [0, 1, 6, 7];

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentProperties {
    id: String,
    name: String,
    added_on: u64,
    completed_on: Option<u64>,
    time_active: u64,
    save_path: String,
    uploaded_total: u64,
    downloaded_total: u64,
    availability: f64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFile {
    id: u32,
    path: String,
    size: u64,
    progress: f64,
    priority: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentTracker {
    url: String,
    tier: i32,
    /// Raw qBittorrent tracker status code. Values beyond the documented
    /// 0-4 have been observed in the wild (e.g. 6 for unreachable), so the
    /// frontend falls back to a generic label for anything unknown.
    status: u32,
    message: String,
    seeds: u64,
    peers: u64,
    leeches: u64,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFilePriority {
    id: u32,
    priority: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct QbittorrentMetadata {
    hash: String,
    info: QbittorrentMetadataInfo,
}

#[derive(Deserialize)]
struct QbittorrentMetadataInfo {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    files: Vec<QbittorrentMetadataFile>,
    #[serde(default)]
    length: Option<u64>,
}

#[derive(Deserialize)]
struct QbittorrentMetadataFile {
    path: String,
    length: u64,
}

impl From<QbittorrentMetadata> for TorrentMetadata {
    fn from(metadata: QbittorrentMetadata) -> Self {
        let name = metadata.info.name.unwrap_or_default();
        let files = if metadata.info.files.is_empty() {
            metadata
                .info
                .length
                .map(|length| TorrentMetadataFile {
                    path: name.clone(),
                    length,
                })
                .into_iter()
                .collect()
        } else {
            metadata
                .info
                .files
                .into_iter()
                .map(|file| TorrentMetadataFile {
                    path: file.path,
                    length: file.length,
                })
                .collect()
        };

        Self {
            hash: metadata.hash,
            name,
            files,
        }
    }
}

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
struct QbittorrentAddOutcome {
    #[serde(default, alias = "success_count")]
    success_count: u32,
    #[serde(default, alias = "failure_count")]
    failure_count: u32,
    #[serde(default, alias = "pending_count")]
    pending_count: u32,
    #[serde(default, alias = "added_torrent_ids")]
    added_torrent_ids: Vec<String>,
}

impl From<QbittorrentAddOutcome> for AddTorrentsOutcome {
    fn from(outcome: QbittorrentAddOutcome) -> Self {
        Self {
            success_count: outcome.success_count,
            failure_count: outcome.failure_count,
            pending_count: outcome.pending_count,
            added_torrent_ids: outcome.added_torrent_ids,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Torrent {
    id: String,
    name: String,
    status: TorrentStatus,
    progress: f64,
    size: u64,
    downloaded: u64,
    down_speed: u64,
    up_speed: u64,
    eta_seconds: Option<u64>,
    ratio: f64,
    seeds: u64,
    peers: u64,
    category: String,
    tags: Vec<String>,
    added_on: u64,
    save_path: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum TorrentStatus {
    Downloading,
    Seeding,
    Paused,
    Checking,
    Stalled,
    Error,
}

#[derive(Deserialize)]
struct QbittorrentTorrent {
    hash: String,
    name: String,
    state: String,
    progress: f64,
    size: i64,
    downloaded: i64,
    dlspeed: i64,
    upspeed: i64,
    eta: i64,
    ratio: f64,
    num_seeds: i64,
    num_leechs: i64,
    category: String,
    tags: String,
    added_on: i64,
    save_path: String,
}

#[derive(Deserialize)]
struct QbittorrentProperties {
    hash: String,
    name: String,
    addition_date: i64,
    completion_date: i64,
    time_elapsed: i64,
    save_path: String,
    total_uploaded: i64,
    total_downloaded: i64,
    availability: f64,
}

#[derive(Deserialize)]
struct QbittorrentTorrentFile {
    #[serde(default)]
    index: Option<u32>,
    name: String,
    size: u64,
    progress: f64,
    priority: u32,
}

#[derive(Deserialize)]
struct QbittorrentTracker {
    url: String,
    tier: i32,
    status: u32,
    #[serde(default)]
    msg: String,
    num_seeds: i64,
    num_peers: i64,
    num_leeches: i64,
}

#[derive(Deserialize)]
struct QbittorrentCategory {
    name: String,
}

#[derive(Debug, PartialEq, Eq)]
enum ConnectionError {
    InvalidConfiguration(String),
    AuthenticationFailed,
    ConnectionFailed(String),
    UnsupportedVersion(String),
    InvalidResponse(String),
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message)
            | Self::ConnectionFailed(message)
            | Self::InvalidResponse(message) => formatter.write_str(message),
            Self::AuthenticationFailed => formatter.write_str(
                "Authentication failed. Check the API key or WebUI username and password.",
            ),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "qBittorrent {version} is not supported. Cloudburst requires qBittorrent 5.2 or newer."
            ),
        }
    }
}

#[tauri::command]
pub async fn connect_qbittorrent(
    input: ConnectionInput,
    manager: State<'_, ConnectionManager>,
    app: tauri::AppHandle,
) -> Result<ConnectionSnapshot, String> {
    let mut active_connection = manager.active.lock().await;
    let (input, profile, secret) = resolve_connection_input(input, &app).await?;
    let (active, snapshot) = establish_connection(input)
        .await
        .map_err(|error| error.to_string())?;
    connection_profile::save(&app, profile, secret).await?;

    *active_connection = Some(active);

    Ok(snapshot)
}

#[tauri::command]
pub async fn restore_saved_qbittorrent(
    manager: State<'_, ConnectionManager>,
    app: tauri::AppHandle,
) -> Result<RestoreOutcome, String> {
    let mut active_connection = manager.active.lock().await;
    let stored = match connection_profile::load(&app).await {
        Ok(Some(stored)) => stored,
        Ok(None) => {
            return Ok(RestoreOutcome {
                profile: None,
                snapshot: None,
                error: None,
            });
        }
        Err(error) => {
            return Ok(RestoreOutcome {
                profile: None,
                snapshot: None,
                error: Some(error),
            });
        }
    };
    let profile = stored.profile.clone();

    Ok(
        match establish_connection(ConnectionInput::from(stored)).await {
            Ok((active, snapshot)) => {
                *active_connection = Some(active);
                RestoreOutcome {
                    profile: Some(profile),
                    snapshot: Some(snapshot),
                    error: None,
                }
            }
            Err(error) => RestoreOutcome {
                profile: Some(profile),
                snapshot: None,
                error: Some(error.to_string()),
            },
        },
    )
}

#[tauri::command]
pub async fn refresh_qbittorrent(
    manager: State<'_, ConnectionManager>,
) -> Result<ConnectionSnapshot, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active.snapshot().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_torrents_paused(
    torrent_ids: Vec<String>,
    paused: bool,
    manager: State<'_, ConnectionManager>,
) -> Result<ConnectionSnapshot, String> {
    let torrent_ids = unique_torrent_ids(torrent_ids);
    if torrent_ids.is_empty() {
        return Err("Select at least one torrent.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .set_paused(&torrent_ids, paused)
        .await
        .map_err(|error| error.to_string())?;
    active.snapshot().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn remove_torrents(
    torrent_ids: Vec<String>,
    delete_files: bool,
    manager: State<'_, ConnectionManager>,
) -> Result<ConnectionSnapshot, String> {
    let torrent_ids = unique_torrent_ids(torrent_ids);
    if torrent_ids.is_empty() {
        return Err("Select at least one torrent.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .remove(&torrent_ids, delete_files)
        .await
        .map_err(|error| error.to_string())?;
    active.snapshot().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn add_torrents(
    input: AddTorrentsInput,
    manager: State<'_, ConnectionManager>,
) -> Result<AddTorrentsOutcome, String> {
    let input = normalize_add_input(input)?;
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .add_torrents(&input)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_default_save_path(
    manager: State<'_, ConnectionManager>,
) -> Result<String, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .default_save_path()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn parse_torrent_metadata(
    files: Vec<AddTorrentFile>,
    manager: State<'_, ConnectionManager>,
) -> Result<Vec<TorrentMetadata>, String> {
    let files: Vec<AddTorrentFile> = files
        .into_iter()
        .filter(|file| !file.name.trim().is_empty() && !file.base64_content.trim().is_empty())
        .collect();
    if files.is_empty() {
        return Err("Choose at least one torrent file.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .parse_metadata(&files)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_torrent_metadata(
    source: String,
    manager: State<'_, ConnectionManager>,
) -> Result<MetadataFetch, String> {
    let source = source.trim().to_string();
    if source.is_empty() {
        return Err("Provide a magnet link or URL.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .fetch_metadata(&source)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_torrent_properties(
    torrent_id: String,
    manager: State<'_, ConnectionManager>,
) -> Result<TorrentProperties, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .properties(&torrent_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_torrent_files(
    torrent_id: String,
    manager: State<'_, ConnectionManager>,
) -> Result<Vec<TorrentFile>, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .files(&torrent_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_torrent_trackers(
    torrent_id: String,
    manager: State<'_, ConnectionManager>,
) -> Result<Vec<TorrentTracker>, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .trackers(&torrent_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_torrent_file_priorities(
    torrent_id: String,
    priorities: Vec<TorrentFilePriority>,
    manager: State<'_, ConnectionManager>,
) -> Result<(), String> {
    let priorities = normalize_file_priorities(priorities)?;

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    // The endpoint accepts one file per request; the WebUI issues the same loop.
    for priority in priorities {
        active
            .client
            .set_file_priority(&torrent_id, priority.id, priority.priority)
            .await
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_torrent_category(
    torrent_ids: Vec<String>,
    category: String,
    manager: State<'_, ConnectionManager>,
) -> Result<(), String> {
    let torrent_ids = unique_torrent_ids(torrent_ids);
    if torrent_ids.is_empty() {
        return Err("Select at least one torrent.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    // An empty name clears the category. setCategory refuses names the
    // instance has not seen, so make sure new categories exist first; an
    // existing category is left untouched.
    let category = category.trim().to_string();
    if category.is_empty() {
        active
            .client
            .set_category(&torrent_ids, "")
            .await
            .map_err(|error| error.to_string())
    }
    else {
        active
            .client
            .set_category_creating_missing(&torrent_ids, &category)
            .await
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub async fn add_torrent_tags(
    torrent_ids: Vec<String>,
    tags: Vec<String>,
    manager: State<'_, ConnectionManager>,
) -> Result<(), String> {
    let torrent_ids = unique_torrent_ids(torrent_ids);
    let tags = normalize_tag_list(tags);
    if torrent_ids.is_empty() {
        return Err("Select at least one torrent.".to_string());
    }
    if tags.is_empty() {
        return Err("Provide at least one tag.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    // addTags creates unknown tags on the instance, so assignment is one call.
    active
        .client
        .add_tags(&torrent_ids, &tags)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn remove_torrent_tags(
    torrent_ids: Vec<String>,
    tags: Vec<String>,
    manager: State<'_, ConnectionManager>,
) -> Result<(), String> {
    let torrent_ids = unique_torrent_ids(torrent_ids);
    let tags = normalize_tag_list(tags);
    if torrent_ids.is_empty() {
        return Err("Select at least one torrent.".to_string());
    }
    if tags.is_empty() {
        return Err("Provide at least one tag.".to_string());
    }

    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .remove_tags(&torrent_ids, &tags)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_categories(
    manager: State<'_, ConnectionManager>,
) -> Result<Vec<String>, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .categories()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn fetch_tags(manager: State<'_, ConnectionManager>) -> Result<Vec<String>, String> {
    let active_connection = manager.active.lock().await;
    let active = active_connection
        .as_ref()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active
        .client
        .tags()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn disconnect_qbittorrent(
    manager: State<'_, ConnectionManager>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut active_connection = manager.active.lock().await;
    connection_profile::clear(&app).await?;
    *active_connection = None;
    Ok(())
}

async fn establish_connection(
    input: ConnectionInput,
) -> Result<(ActiveConnection, ConnectionSnapshot), ConnectionError> {
    let client = QbittorrentClient::new(input)?;
    let version = client.version().await?;

    if !is_supported_version(&version) {
        return Err(ConnectionError::UnsupportedVersion(version));
    }

    let active = ActiveConnection { client, version };
    let snapshot = active.snapshot().await?;
    Ok((active, snapshot))
}

async fn resolve_connection_input(
    input: ConnectionInput,
    app: &tauri::AppHandle,
) -> Result<(ConnectionInput, ConnectionProfile, String), String> {
    let profile = input.profile().map_err(|error| error.to_string())?;
    let supplied_secret = input.secret();
    let secret = if supplied_secret.is_empty() {
        let stored = connection_profile::load(app)
            .await?
            .ok_or_else(|| missing_credential_message(input.authentication_mode))?;
        if stored.profile != profile {
            return Err(missing_credential_message(input.authentication_mode));
        }
        stored.secret
    } else {
        supplied_secret
    };
    let resolved = input.with_secret(secret.clone());

    Ok((resolved, profile, secret))
}

fn missing_credential_message(mode: AuthenticationMode) -> String {
    match mode {
        AuthenticationMode::ApiKey => "Enter a qBittorrent API key.".to_string(),
        AuthenticationMode::Credentials => "Enter the qBittorrent WebUI password.".to_string(),
    }
}

impl ActiveConnection {
    async fn snapshot(&self) -> Result<ConnectionSnapshot, ConnectionError> {
        Ok(ConnectionSnapshot {
            endpoint: self.client.display_endpoint(),
            version: self.version.clone(),
            torrents: self.client.torrents().await?,
        })
    }
}

impl ConnectionInput {
    fn profile(&self) -> Result<ConnectionProfile, ConnectionError> {
        let endpoint = normalize_endpoint(&self.endpoint)?
            .as_str()
            .trim_end_matches('/')
            .to_string();
        let username = match self.authentication_mode {
            AuthenticationMode::ApiKey => None,
            AuthenticationMode::Credentials => {
                let username = self
                    .username
                    .as_ref()
                    .filter(|username| !username.trim().is_empty())
                    .ok_or_else(|| {
                        ConnectionError::InvalidConfiguration(
                            "Enter the qBittorrent WebUI username.".to_string(),
                        )
                    })?;
                Some(username.clone())
            }
        };

        Ok(ConnectionProfile {
            endpoint,
            authentication_mode: self.authentication_mode,
            username,
        })
    }

    fn secret(&self) -> String {
        match self.authentication_mode {
            AuthenticationMode::ApiKey => self.api_key.clone().unwrap_or_default(),
            AuthenticationMode::Credentials => self.password.clone().unwrap_or_default(),
        }
    }

    fn with_secret(mut self, secret: String) -> Self {
        match self.authentication_mode {
            AuthenticationMode::ApiKey => self.api_key = Some(secret),
            AuthenticationMode::Credentials => self.password = Some(secret),
        }
        self
    }
}

impl From<StoredConnection> for ConnectionInput {
    fn from(stored: StoredConnection) -> Self {
        let (api_key, password) = match stored.profile.authentication_mode {
            AuthenticationMode::ApiKey => (Some(stored.secret), None),
            AuthenticationMode::Credentials => (None, Some(stored.secret)),
        };

        Self {
            endpoint: stored.profile.endpoint,
            authentication_mode: stored.profile.authentication_mode,
            api_key,
            username: stored.profile.username,
            password,
        }
    }
}

impl QbittorrentClient {
    fn new(input: ConnectionInput) -> Result<Self, ConnectionError> {
        let endpoint = normalize_endpoint(&input.endpoint)?;
        let authentication = Authentication::from_input(input)?;
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| {
                ConnectionError::InvalidConfiguration(format!(
                    "Failed to configure the qBittorrent HTTP client: {error}"
                ))
            })?;

        Ok(Self {
            http,
            endpoint,
            authentication,
        })
    }

    async fn version(&self) -> Result<String, ConnectionError> {
        let response = self.get("api/v2/app/version").await?;
        let version = response.text().await.map_err(|error| {
            ConnectionError::InvalidResponse(format!(
                "qBittorrent returned an unreadable version response: {error}"
            ))
        })?;
        let version = version.trim().trim_start_matches('v').to_string();

        parse_version(&version).ok_or_else(|| {
            ConnectionError::InvalidResponse(format!(
                "qBittorrent returned an unrecognized version: {version}"
            ))
        })?;

        Ok(version)
    }

    async fn torrents(&self) -> Result<Vec<Torrent>, ConnectionError> {
        let response = self.get("api/v2/torrents/info").await?;
        let torrents = response
            .json::<Vec<QbittorrentTorrent>>()
            .await
            .map_err(|error| {
                ConnectionError::InvalidResponse(format!(
                    "qBittorrent returned an unreadable torrent list: {error}"
                ))
            })?;

        Ok(torrents.into_iter().map(Torrent::from).collect())
    }

    async fn set_paused(
        &self,
        torrent_ids: &[String],
        paused: bool,
    ) -> Result<(), ConnectionError> {
        let path = if paused {
            "api/v2/torrents/stop"
        } else {
            "api/v2/torrents/start"
        };
        let form = [("hashes", torrent_ids.join("|"))];
        self.post_form(path, &form).await?;
        Ok(())
    }

    async fn remove(
        &self,
        torrent_ids: &[String],
        delete_files: bool,
    ) -> Result<(), ConnectionError> {
        let form = [
            ("hashes", torrent_ids.join("|")),
            ("deleteFiles", delete_files.to_string()),
        ];
        self.post_form("api/v2/torrents/delete", &form).await?;
        Ok(())
    }

    async fn default_save_path(&self) -> Result<String, ConnectionError> {
        let response = self.get("api/v2/app/defaultSavePath").await?;
        let path = response.text().await.map_err(|error| {
            ConnectionError::InvalidResponse(format!(
                "qBittorrent returned an unreadable default save path: {error}"
            ))
        })?;
        Ok(path.trim().to_string())
    }

    async fn parse_metadata(
        &self,
        files: &[AddTorrentFile],
    ) -> Result<Vec<TorrentMetadata>, ConnectionError> {
        let url = self.api_url("api/v2/torrents/parseMetadata")?;
        let mut form = Form::new();
        // qBittorrent keys parsed uploads by the part filename, so use the
        // WebUI's scheme of sequential dummy names to avoid collisions.
        for (index, file) in files.iter().enumerate() {
            let content = BASE64_STANDARD
                .decode(file.base64_content.trim())
                .map_err(|error| {
                    ConnectionError::InvalidConfiguration(format!(
                        "Could not read the torrent file {}: {error}",
                        file.name
                    ))
                })?;
            let part = Part::bytes(content)
                .file_name(index.to_string())
                .mime_str("application/x-bittorrent")
                .map_err(|error| {
                    ConnectionError::InvalidConfiguration(format!(
                        "Could not prepare the torrent file {}: {error}",
                        file.name
                    ))
                })?;
            form = form.part("file", part);
        }

        let request = self.authentication.apply(self.http.post(url).multipart(form));
        let response = request.send().await.map_err(|error| {
            ConnectionError::ConnectionFailed(format!(
                "Could not reach qBittorrent at {}: {error}",
                self.display_endpoint()
            ))
        })?;

        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ConnectionError::AuthenticationFailed)
            }
            status if status.is_success() => response
                .json::<Vec<QbittorrentMetadata>>()
                .await
                .map(|list| list.into_iter().map(TorrentMetadata::from).collect())
                .map_err(|error| {
                    ConnectionError::InvalidResponse(format!(
                        "qBittorrent returned an unreadable metadata response: {error}"
                    ))
                }),
            status => Err(ConnectionError::InvalidResponse(format!(
                "qBittorrent could not read the torrent file (HTTP {status})."
            ))),
        }
    }

    async fn fetch_metadata(&self, source: &str) -> Result<MetadataFetch, ConnectionError> {
        let form = [("source", source.to_string())];
        let url = self.api_url("api/v2/torrents/fetchMetadata")?;
        let request = self.authentication.apply(self.http.post(url).form(&form));
        let response = request.send().await.map_err(|error| {
            ConnectionError::ConnectionFailed(format!(
                "Could not reach qBittorrent at {}: {error}",
                self.display_endpoint()
            ))
        })?;

        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ConnectionError::AuthenticationFailed)
            }
            // 200: metadata is available now. 202: the instance is still
            // fetching it from the swarm; poll again.
            StatusCode::OK => response
                .json::<QbittorrentMetadata>()
                .await
                .map(|metadata| MetadataFetch::Ready {
                    metadata: metadata.into(),
                })
                .map_err(|error| {
                    ConnectionError::InvalidResponse(format!(
                        "qBittorrent returned an unreadable metadata response: {error}"
                    ))
                }),
            StatusCode::ACCEPTED => Ok(MetadataFetch::Pending),
            status => Err(ConnectionError::InvalidResponse(format!(
                "qBittorrent could not fetch the metadata (HTTP {status})."
            ))),
        }
    }

    async fn add_torrents(
        &self,
        input: &AddTorrentsInput,
    ) -> Result<AddTorrentsOutcome, ConnectionError> {
        let url = self.api_url("api/v2/torrents/add")?;
        let mut form = Form::new();

        if !input.urls.is_empty() {
            form = form.text("urls", input.urls.join("\n"));
        }
        for file in &input.files {
            let content = BASE64_STANDARD
                .decode(file.base64_content.trim())
                .map_err(|error| {
                    ConnectionError::InvalidConfiguration(format!(
                        "Could not read the torrent file {}: {error}",
                        file.name
                    ))
                })?;
            let part = Part::bytes(content)
                .file_name(file.name.clone())
                .mime_str("application/x-bittorrent")
                .map_err(|error| {
                    ConnectionError::InvalidConfiguration(format!(
                        "Could not prepare the torrent file {}: {error}",
                        file.name
                    ))
                })?;
            form = form.part("torrents", part);
        }
        if let Some(category) = input.category.as_deref().map(str::trim).filter(|category| !category.is_empty()) {
            form = form.text("category", category.to_string());
        }
        if let Some(save_path) = input.save_path.as_deref().map(str::trim).filter(|save_path| !save_path.is_empty()) {
            form = form.text("savepath", save_path.to_string());
        }
        form = form.text("contentLayout", input.content_layout.as_qbittorrent_value());
        if let Some(priorities) = &input.file_priorities {
            let joined = priorities
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            form = form.text("filePriorities", joined);
        }

        let request = self.authentication.apply(self.http.post(url).multipart(form));
        let response = request.send().await.map_err(|error| {
            ConnectionError::ConnectionFailed(format!(
                "Could not reach qBittorrent at {}: {error}",
                self.display_endpoint()
            ))
        })?;

        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ConnectionError::AuthenticationFailed)
            }
            StatusCode::UNSUPPORTED_MEDIA_TYPE => Err(ConnectionError::InvalidResponse(
                "qBittorrent rejected the torrent data as unreadable.".to_string(),
            )),
            // 200: added synchronously. 202: URLs/magnets still being fetched.
            // 409: nothing was added, usually duplicates. Some conflicts carry
            // the outcome body; others answer with plain text.
            status if status.is_success() || status == StatusCode::CONFLICT => {
                let text = response.text().await.map_err(|error| {
                    ConnectionError::InvalidResponse(format!(
                        "qBittorrent returned an unreadable add response: {error}"
                    ))
                })?;
                if text.trim().is_empty() {
                    return Ok(AddTorrentsOutcome::default());
                }
                match serde_json::from_str::<QbittorrentAddOutcome>(&text) {
                    Ok(outcome) => Ok(outcome.into()),
                    // A duplicate add answers HTTP 409 with a plain-text body,
                    // which still means nothing was added.
                    Err(_) if status == StatusCode::CONFLICT => Ok(AddTorrentsOutcome::default()),
                    Err(error) => Err(ConnectionError::InvalidResponse(format!(
                        "qBittorrent returned an unreadable add response: {error}"
                    ))),
                }
            }
            status => Err(ConnectionError::ConnectionFailed(format!(
                "qBittorrent at {} returned HTTP {status}.",
                self.display_endpoint()
            ))),
        }
    }

    async fn properties(&self, torrent_id: &str) -> Result<TorrentProperties, ConnectionError> {
        let response = self
            .get_with_query("api/v2/torrents/properties", &[("hash", torrent_id)])
            .await?;
        let properties = response
            .json::<QbittorrentProperties>()
            .await
            .map_err(|error| {
                ConnectionError::InvalidResponse(format!(
                    "qBittorrent returned an unreadable torrent properties response: {error}"
                ))
            })?;

        Ok(TorrentProperties {
            id: properties.hash,
            name: properties.name,
            added_on: non_negative(properties.addition_date),
            completed_on: (properties.completion_date >= 0)
                .then_some(properties.completion_date as u64),
            time_active: non_negative(properties.time_elapsed),
            save_path: properties.save_path,
            uploaded_total: non_negative(properties.total_uploaded),
            downloaded_total: non_negative(properties.total_downloaded),
            availability: properties.availability,
        })
    }

    async fn files(&self, torrent_id: &str) -> Result<Vec<TorrentFile>, ConnectionError> {
        let response = self
            .get_with_query("api/v2/torrents/files", &[("hash", torrent_id)])
            .await?;
        let files = response
            .json::<Vec<QbittorrentTorrentFile>>()
            .await
            .map_err(|error| {
                ConnectionError::InvalidResponse(format!(
                    "qBittorrent returned an unreadable torrent file list: {error}"
                ))
            })?;

        Ok(files
            .into_iter()
            .enumerate()
            .map(|(position, file)| TorrentFile {
                id: file.index.unwrap_or(position as u32),
                path: file.name,
                size: file.size,
                progress: file.progress,
                priority: file.priority,
            })
            .collect())
    }

    async fn trackers(&self, torrent_id: &str) -> Result<Vec<TorrentTracker>, ConnectionError> {
        let response = self
            .get_with_query("api/v2/torrents/trackers", &[("hash", torrent_id)])
            .await?;
        let trackers = response
            .json::<Vec<QbittorrentTracker>>()
            .await
            .map_err(|error| {
                ConnectionError::InvalidResponse(format!(
                    "qBittorrent returned an unreadable tracker list: {error}"
                ))
            })?;

        Ok(trackers
            .into_iter()
            .map(|tracker| TorrentTracker {
                url: tracker.url,
                tier: tracker.tier,
                status: tracker.status,
                message: tracker.msg,
                seeds: non_negative(tracker.num_seeds),
                peers: non_negative(tracker.num_peers),
                leeches: non_negative(tracker.num_leeches),
            })
            .collect())
    }

    async fn set_file_priority(
        &self,
        torrent_id: &str,
        file_id: u32,
        priority: u32,
    ) -> Result<(), ConnectionError> {
        let form = [
            ("hash", torrent_id.to_string()),
            ("id", file_id.to_string()),
            ("priority", priority.to_string()),
        ];
        self.post_form("api/v2/torrents/filePrio", &form).await?;
        Ok(())
    }

    async fn categories(&self) -> Result<Vec<String>, ConnectionError> {
        let response = self.get("api/v2/torrents/categories").await?;
        let categories = response
            .json::<std::collections::HashMap<String, QbittorrentCategory>>()
            .await
            .map_err(|error| {
                ConnectionError::InvalidResponse(format!(
                    "qBittorrent returned an unreadable category list: {error}"
                ))
            })?;

        let mut names: Vec<String> = categories.into_iter().map(|(_, category)| category.name).collect();
        names.sort();
        Ok(names)
    }

    async fn tags(&self) -> Result<Vec<String>, ConnectionError> {
        let response = self.get("api/v2/torrents/tags").await?;
        let mut tags: Vec<String> = response.json().await.map_err(|error| {
            ConnectionError::InvalidResponse(format!(
                "qBittorrent returned an unreadable tag list: {error}"
            ))
        })?;
        tags.sort();
        Ok(tags)
    }

    async fn create_category(&self, category: &str) -> Result<(), ConnectionError> {
        let form = [("category", category.to_string())];
        self.post_form("api/v2/torrents/createCategory", &form)
            .await?;
        Ok(())
    }

    async fn set_category(
        &self,
        torrent_ids: &[String],
        category: &str,
    ) -> Result<(), ConnectionError> {
        let form = [
            ("hashes", torrent_ids.join("|")),
            ("category", category.to_string()),
        ];
        self.post_form("api/v2/torrents/setCategory", &form).await?;
        Ok(())
    }

    /// setCategory refuses names the instance has not seen, so make sure the
    /// category exists first. An existing category is left untouched; the
    /// assignment call is the real gate either way.
    async fn set_category_creating_missing(
        &self,
        torrent_ids: &[String],
        category: &str,
    ) -> Result<(), ConnectionError> {
        let _ = self.create_category(category).await;
        self.set_category(torrent_ids, category).await
    }

    async fn add_tags(&self, torrent_ids: &[String], tags: &[String]) -> Result<(), ConnectionError> {
        let form = [
            ("hashes", torrent_ids.join("|")),
            ("tags", tags.join(",")),
        ];
        self.post_form("api/v2/torrents/addTags", &form).await?;
        Ok(())
    }

    async fn remove_tags(
        &self,
        torrent_ids: &[String],
        tags: &[String],
    ) -> Result<(), ConnectionError> {
        let form = [
            ("hashes", torrent_ids.join("|")),
            ("tags", tags.join(",")),
        ];
        self.post_form("api/v2/torrents/removeTags", &form).await?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response, ConnectionError> {
        let url = self.api_url(path)?;
        let request = self.authentication.apply(self.http.get(url));
        self.send(request).await
    }

    async fn get_with_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<reqwest::Response, ConnectionError> {
        let mut url = self.api_url(path)?;
        for (key, value) in query {
            url.query_pairs_mut().append_pair(key, value);
        }
        let request = self.authentication.apply(self.http.get(url));
        self.send(request).await
    }

    async fn post_form(
        &self,
        path: &str,
        form: &[(&str, String)],
    ) -> Result<reqwest::Response, ConnectionError> {
        let url = self.api_url(path)?;
        let request = self.authentication.apply(self.http.post(url).form(form));
        self.send(request).await
    }

    fn api_url(&self, path: &str) -> Result<Url, ConnectionError> {
        let url = self.endpoint.join(path).map_err(|error| {
            ConnectionError::InvalidConfiguration(format!(
                "Failed to build a qBittorrent API URL: {error}"
            ))
        })?;
        Ok(url)
    }

    async fn send(&self, request: RequestBuilder) -> Result<reqwest::Response, ConnectionError> {
        let response = request.send().await.map_err(|error| {
            ConnectionError::ConnectionFailed(format!(
                "Could not reach qBittorrent at {}: {error}",
                self.display_endpoint()
            ))
        })?;

        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ConnectionError::AuthenticationFailed)
            }
            status if !status.is_success() => Err(ConnectionError::ConnectionFailed(format!(
                "qBittorrent at {} returned HTTP {status}.",
                self.display_endpoint()
            ))),
            _ => Ok(response),
        }
    }

    fn display_endpoint(&self) -> String {
        self.endpoint.as_str().trim_end_matches('/').to_string()
    }
}

impl Authentication {
    fn from_input(input: ConnectionInput) -> Result<Self, ConnectionError> {
        match input.authentication_mode {
            AuthenticationMode::ApiKey => {
                let api_key = input.api_key.unwrap_or_default().trim().to_string();
                let valid = api_key.len() == 32
                    && api_key.starts_with("qbt_")
                    && api_key[4..]
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric());

                if !valid {
                    return Err(ConnectionError::InvalidConfiguration(
                        "The API key must start with qbt_ and contain 32 characters.".to_string(),
                    ));
                }

                Ok(Self::ApiKey(api_key))
            }
            AuthenticationMode::Credentials => match (input.username, input.password) {
                (Some(username), Some(password))
                    if !username.trim().is_empty() && !password.is_empty() =>
                {
                    Ok(Self::Credentials { username, password })
                }
                _ => Err(ConnectionError::InvalidConfiguration(
                    "Enter the qBittorrent WebUI username and password.".to_string(),
                )),
            },
        }
    }

    fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        match self {
            Self::ApiKey(api_key) => request.bearer_auth(api_key),
            Self::Credentials { username, password } => {
                request.basic_auth(username, Some(password))
            }
        }
    }
}

impl From<QbittorrentTorrent> for Torrent {
    fn from(torrent: QbittorrentTorrent) -> Self {
        Self {
            id: torrent.hash,
            name: torrent.name,
            status: map_status(&torrent.state),
            progress: (torrent.progress.clamp(0.0, 1.0) * 1000.0).round() / 10.0,
            size: non_negative(torrent.size),
            downloaded: non_negative(torrent.downloaded),
            down_speed: non_negative(torrent.dlspeed),
            up_speed: non_negative(torrent.upspeed),
            eta_seconds: (torrent.eta >= 0).then_some(torrent.eta as u64),
            ratio: torrent.ratio.max(0.0),
            seeds: non_negative(torrent.num_seeds),
            peers: non_negative(torrent.num_leechs),
            category: torrent.category,
            tags: torrent
                .tags
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            added_on: non_negative(torrent.added_on),
            save_path: torrent.save_path,
        }
    }
}

fn normalize_endpoint(input: &str) -> Result<Url, ConnectionError> {
    let mut endpoint = Url::parse(input.trim()).map_err(|_| {
        ConnectionError::InvalidConfiguration(
            "Enter a valid qBittorrent WebUI URL, including http:// or https://.".to_string(),
        )
    })?;

    if !matches!(endpoint.scheme(), "http" | "https") || endpoint.host_str().is_none() {
        return Err(ConnectionError::InvalidConfiguration(
            "The qBittorrent WebUI URL must use http:// or https:// and include a host."
                .to_string(),
        ));
    }
    if endpoint.scheme() == "http" && !is_loopback_endpoint(&endpoint) {
        return Err(ConnectionError::InvalidConfiguration(
            "Remote qBittorrent connections must use HTTPS. Plain HTTP is allowed only for localhost."
                .to_string(),
        ));
    }
    if !endpoint.username().is_empty() || endpoint.password().is_some() {
        return Err(ConnectionError::InvalidConfiguration(
            "Do not put credentials in the qBittorrent WebUI URL.".to_string(),
        ));
    }
    if endpoint.query().is_some() || endpoint.fragment().is_some() {
        return Err(ConnectionError::InvalidConfiguration(
            "The qBittorrent WebUI URL cannot contain a query or fragment.".to_string(),
        ));
    }

    let path = format!("{}/", endpoint.path().trim_end_matches('/'));
    endpoint.set_path(&path);
    Ok(endpoint)
}

fn is_loopback_endpoint(endpoint: &Url) -> bool {
    endpoint.host_str().is_some_and(|host| {
        let host = host.trim_start_matches('[').trim_end_matches(']');
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    })
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let numeric = version
        .trim_start_matches('v')
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .next()?;
    let mut components = numeric.split('.');
    let major = components.next()?.parse().ok()?;
    let minor = components.next()?.parse().ok()?;
    let patch = components.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn is_supported_version(version: &str) -> bool {
    parse_version(version).is_some_and(|version| version >= MINIMUM_VERSION)
}

fn map_status(state: &str) -> TorrentStatus {
    match state {
        "pausedUP" | "pausedDL" | "stoppedUP" | "stoppedDL" => TorrentStatus::Paused,
        "checkingUP" | "checkingDL" | "checkingResumeData" | "moving" => TorrentStatus::Checking,
        "stalledUP" | "stalledDL" => TorrentStatus::Stalled,
        "error" | "missingFiles" | "unknown" => TorrentStatus::Error,
        "uploading" | "queuedUP" | "forcedUP" => TorrentStatus::Seeding,
        _ => TorrentStatus::Downloading,
    }
}

fn non_negative(value: i64) -> u64 {
    value.max(0) as u64
}

fn unique_torrent_ids(torrent_ids: Vec<String>) -> Vec<String> {
    torrent_ids.into_iter().fold(Vec::new(), |mut unique, id| {
        let id = id.trim().to_string();
        if !id.is_empty() && !unique.contains(&id) {
            unique.push(id);
        }
        unique
    })
}

fn normalize_tag_list(tags: Vec<String>) -> Vec<String> {
    tags.iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .fold(Vec::new(), |mut unique, tag| {
            if !unique.contains(&tag) {
                unique.push(tag);
            }
            unique
        })
}

fn normalize_file_priorities(
    priorities: Vec<TorrentFilePriority>,
) -> Result<Vec<TorrentFilePriority>, String> {
    let mut normalized: Vec<TorrentFilePriority> = Vec::new();
    for priority in priorities {
        if !TORRENT_FILE_PRIORITIES.contains(&priority.priority) {
            return Err(
                "File priority must be 0 (skip), 1 (normal), 6 (high), or 7 (maximum)."
                    .to_string(),
            );
        }
        if !normalized.iter().any(|existing| existing.id == priority.id) {
            normalized.push(priority);
        }
    }

    if normalized.is_empty() {
        return Err("Select at least one file.".to_string());
    }

    Ok(normalized)
}

fn normalize_add_input(input: AddTorrentsInput) -> Result<AddTorrentsInput, String> {
    let urls: Vec<String> = input
        .urls
        .iter()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .fold(Vec::new(), |mut unique, url| {
            if !unique.contains(&url) {
                unique.push(url);
            }
            unique
        });
    let files: Vec<AddTorrentFile> = input
        .files
        .into_iter()
        .filter(|file| {
            !file.name.trim().is_empty() && !file.base64_content.trim().is_empty()
        })
        .collect();

    if urls.is_empty() && files.is_empty() {
        return Err("Provide a magnet link, URL, or .torrent file.".to_string());
    }

    let file_priorities = match input.file_priorities {
        Some(priorities) if !priorities.is_empty() => {
            if urls.len() != 1 || !files.is_empty() {
                return Err(
                    "File priorities can only apply to a single torrent added by link or hash."
                        .to_string(),
                );
            }
            if !priorities
                .iter()
                .all(|priority| TORRENT_FILE_PRIORITIES.contains(priority))
            {
                return Err(
                    "File priority must be 0 (skip), 1 (normal), 6 (high), or 7 (maximum)."
                        .to_string(),
                );
            }
            Some(priorities)
        }
        _ => None,
    };

    Ok(AddTorrentsInput {
        urls,
        files,
        category: input
            .category
            .map(|category| category.trim().to_string())
            .filter(|category| !category.is_empty()),
        save_path: input
            .save_path
            .map(|save_path| save_path.trim().to_string())
            .filter(|save_path| !save_path.is_empty()),
        content_layout: input.content_layout,
        file_priorities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    #[test]
    fn normalizes_root_and_reverse_proxy_endpoints() {
        assert_eq!(
            normalize_endpoint("http://localhost:8080")
                .unwrap()
                .as_str(),
            "http://localhost:8080/"
        );
        assert_eq!(
            normalize_endpoint("https://example.test/qbittorrent")
                .unwrap()
                .join("api/v2/app/version")
                .unwrap()
                .as_str(),
            "https://example.test/qbittorrent/api/v2/app/version"
        );
    }

    #[test]
    fn rejects_unsafe_or_incomplete_endpoints() {
        assert!(normalize_endpoint("localhost:8080").is_err());
        assert!(normalize_endpoint("ftp://localhost:8080").is_err());
        assert!(normalize_endpoint("http://admin:secret@localhost:8080").is_err());
        assert!(normalize_endpoint("http://localhost:8080?secret=value").is_err());
    }

    #[test]
    fn requires_https_outside_loopback_addresses() {
        assert!(normalize_endpoint("http://localhost:8080").is_ok());
        assert!(normalize_endpoint("http://127.0.0.1:8080").is_ok());
        assert!(normalize_endpoint("http://[::1]:8080").is_ok());
        assert!(normalize_endpoint("https://qbittorrent.example.test").is_ok());
        assert!(normalize_endpoint("http://qbittorrent.example.test").is_err());
        assert!(normalize_endpoint("http://192.168.1.50:8080").is_err());
    }

    #[test]
    fn applies_the_documented_version_floor() {
        assert!(!is_supported_version("5.1.9"));
        assert!(is_supported_version("5.2.0"));
        assert!(is_supported_version("v5.2.1alpha1"));
        assert!(is_supported_version("6.0.0"));
        assert!(!is_supported_version("not-a-version"));
    }

    #[test]
    fn maps_qbittorrent_states_to_cloudburst_states() {
        assert_eq!(map_status("downloading"), TorrentStatus::Downloading);
        assert_eq!(map_status("uploading"), TorrentStatus::Seeding);
        assert_eq!(map_status("stoppedDL"), TorrentStatus::Paused);
        assert_eq!(map_status("checkingUP"), TorrentStatus::Checking);
        assert_eq!(map_status("stalledDL"), TorrentStatus::Stalled);
        assert_eq!(map_status("missingFiles"), TorrentStatus::Error);
    }

    #[test]
    fn normalizes_selected_torrent_ids() {
        assert_eq!(
            unique_torrent_ids(vec![
                " abc123 ".to_string(),
                "".to_string(),
                "abc123".to_string(),
                "def456".to_string(),
            ]),
            ["abc123", "def456"]
        );
    }

    #[test]
    fn normalizes_add_input_sources() {
        let input = normalize_add_input(AddTorrentsInput {
            urls: vec![
                " magnet:?xt=urn:btih:abc ".to_string(),
                String::new(),
                "magnet:?xt=urn:btih:abc".to_string(),
            ],
            files: vec![
                AddTorrentFile {
                    name: "keep.torrent".to_string(),
                    base64_content: "AAAA".to_string(),
                },
                AddTorrentFile {
                    name: "  ".to_string(),
                    base64_content: "AAAA".to_string(),
                },
                AddTorrentFile {
                    name: "empty.torrent".to_string(),
                    base64_content: " ".to_string(),
                },
            ],
            category: Some("  Linux  ".to_string()),
            save_path: Some(" ".to_string()),
            content_layout: AddContentLayout::Subfolder,
            file_priorities: None,
        })
        .unwrap();

        assert_eq!(input.urls, ["magnet:?xt=urn:btih:abc"]);
        assert_eq!(input.files.len(), 1);
        assert_eq!(input.files[0].name, "keep.torrent");
        assert_eq!(input.category.as_deref(), Some("Linux"));
        assert_eq!(input.save_path, None);
        assert_eq!(input.content_layout, AddContentLayout::Subfolder);

        assert!(normalize_add_input(AddTorrentsInput {
            urls: vec![" ".to_string()],
            files: Vec::new(),
            category: None,
            save_path: None,
            content_layout: AddContentLayout::Original,
            file_priorities: None,
        })
        .is_err());
    }

    #[test]
    fn parses_the_qbittorrent_add_outcome() {
        let outcome: QbittorrentAddOutcome = serde_json::from_str(
            r#"{"success_count":2,"failure_count":1,"pending_count":3,"added_torrent_ids":["a","b"]}"#,
        )
        .unwrap();
        let public: AddTorrentsOutcome = outcome.into();

        assert_eq!(public.success_count, 2);
        assert_eq!(public.failure_count, 1);
        assert_eq!(public.pending_count, 3);
        assert_eq!(public.added_torrent_ids, ["a", "b"]);

        let serialized = serde_json::to_value(&public).unwrap();
        assert_eq!(serialized["addedTorrentIds"], serde_json::json!(["a", "b"]));
        assert_eq!(serialized["successCount"], serde_json::json!(2));
    }

    #[test]
    fn fetches_a_read_only_snapshot_with_bearer_authentication() {
        let torrent_json = r#"[{
            "hash":"abc123",
            "name":"Debian ISO",
            "state":"downloading",
            "progress":0.625,
            "size":4096,
            "downloaded":2560,
            "dlspeed":1024,
            "upspeed":128,
            "eta":90,
            "ratio":0.5,
            "num_seeds":12,
            "num_leechs":3,
            "category":"Linux",
            "tags":"iso, priority",
            "added_on":1700000000,
            "save_path":"C:/Downloads"
        }]"#;
        let (endpoint, requests, server) = serve_responses(vec!["v5.2.1", torrent_json]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        let (version, torrents) = tauri::async_runtime::block_on(async {
            (
                client.version().await.unwrap(),
                client.torrents().await.unwrap(),
            )
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(version, "5.2.1");
        assert_eq!(torrents.len(), 1);
        assert_eq!(torrents[0].name, "Debian ISO");
        assert_eq!(torrents[0].progress, 62.5);
        assert_eq!(torrents[0].tags, ["iso", "priority"]);
        assert!(requests
            .iter()
            .all(|request| request
                .contains("authorization: Bearer qbt_0000000000000000000000000000")));
        assert!(requests[0].starts_with("GET /api/v2/app/version "));
        assert!(requests[1].starts_with("GET /api/v2/torrents/info "));
    }

    #[test]
    fn supports_username_and_password_authentication() {
        let (endpoint, requests, server) = serve_responses(vec!["v5.2.0", "[]"]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::Credentials,
            api_key: None,
            username: Some("admin".to_string()),
            password: Some("secret".to_string()),
        })
        .unwrap();

        tauri::async_runtime::block_on(async {
            client.version().await.unwrap();
            client.torrents().await.unwrap();
        });
        server.join().unwrap();

        assert!(requests
            .iter()
            .all(|request| request.contains("authorization: Basic YWRtaW46c2VjcmV0")));
    }

    #[test]
    fn starts_and_stops_selected_torrents() {
        let (endpoint, requests, server) = serve_responses(vec!["Ok.", "Ok."]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let torrent_ids = vec!["abc123".to_string(), "def456".to_string()];

        tauri::async_runtime::block_on(async {
            client.set_paused(&torrent_ids, true).await.unwrap();
            client.set_paused(&torrent_ids, false).await.unwrap();
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(requests[0].starts_with("POST /api/v2/torrents/stop "));
        assert!(requests[1].starts_with("POST /api/v2/torrents/start "));
        assert!(requests
            .iter()
            .all(|request| request.contains("hashes=abc123%7Cdef456")));
        assert!(requests
            .iter()
            .all(|request| request
                .contains("authorization: Bearer qbt_0000000000000000000000000000")));
    }

    #[test]
    fn removes_selected_torrents_with_and_without_files() {
        let (endpoint, requests, server) = serve_responses(vec!["Ok.", "Ok."]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let torrent_ids = vec!["abc123".to_string(), "def456".to_string()];

        tauri::async_runtime::block_on(async {
            client.remove(&torrent_ids, false).await.unwrap();
            client.remove(&torrent_ids, true).await.unwrap();
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(requests[0].starts_with("POST /api/v2/torrents/delete "));
        assert!(requests[0].contains("hashes=abc123%7Cdef456"));
        assert!(requests[0].contains("deleteFiles=false"));
        assert!(requests[1].starts_with("POST /api/v2/torrents/delete "));
        assert!(requests[1].contains("deleteFiles=true"));
        assert!(requests
            .iter()
            .all(|request| request
                .contains("authorization: Bearer qbt_0000000000000000000000000000")));
    }

    #[test]
    fn adds_torrents_with_links_files_and_options() {
        let outcome_json =
            r#"{"success_count":1,"failure_count":0,"pending_count":0,"added_torrent_ids":["abc123"]}"#;
        let (endpoint, requests, server) = serve_responses(vec![outcome_json]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let input = AddTorrentsInput {
            urls: vec!["magnet:?xt=urn:btih:abc123".to_string()],
            files: vec![AddTorrentFile {
                name: "Debian.torrent".to_string(),
                base64_content: BASE64_STANDARD.encode(b"cloudburst-test-torrent-payload"),
            }],
            category: Some("Linux".to_string()),
            save_path: Some("C:/Downloads/Finished".to_string()),
            content_layout: AddContentLayout::Subfolder,
            file_priorities: None,
        };

        let outcome =
            tauri::async_runtime::block_on(async { client.add_torrents(&input).await.unwrap() });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();
        let request = &requests[0];

        assert_eq!(outcome.success_count, 1);
        assert_eq!(outcome.added_torrent_ids, ["abc123"]);
        assert!(request.starts_with("POST /api/v2/torrents/add "));
        assert!(request.contains("multipart/form-data"));
        assert!(request.contains("name=\"urls\""));
        assert!(request.contains("magnet:?xt=urn:btih:abc123"));
        assert!(request.contains("filename=\"Debian.torrent\""));
        assert!(request.contains("cloudburst-test-torrent-payload"));
        assert!(request.contains("name=\"category\""));
        assert!(request.contains("Linux"));
        assert!(request.contains("name=\"savepath\""));
        assert!(request.contains("C:/Downloads/Finished"));
        assert!(request.contains("name=\"contentLayout\""));
        assert!(request.contains("Subfolder"));
        assert!(request
            .contains("authorization: Bearer qbt_0000000000000000000000000000"));
    }

    #[test]
    fn reports_the_conflict_outcome_when_nothing_is_added() {
        let conflict_json =
            r#"{"success_count":0,"failure_count":2,"pending_count":0,"added_torrent_ids":[]}"#;
        let (endpoint, requests, server) = serve_status_responses(vec![(409, conflict_json)]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let input = AddTorrentsInput {
            urls: vec!["magnet:?xt=urn:btih:duplicate".to_string()],
            files: Vec::new(),
            category: None,
            save_path: None,
            content_layout: AddContentLayout::Original,
            file_priorities: None,
        };

        let outcome =
            tauri::async_runtime::block_on(async { client.add_torrents(&input).await.unwrap() });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(outcome.success_count, 0);
        assert_eq!(outcome.failure_count, 2);
        assert!(requests[0].starts_with("POST /api/v2/torrents/add "));
    }

    #[test]
    fn treats_a_plain_text_conflict_as_nothing_added() {
        let (endpoint, requests, server) = serve_status_responses(vec![(409, "Conflict")]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let input = AddTorrentsInput {
            urls: vec!["magnet:?xt=urn:btih:duplicate".to_string()],
            files: Vec::new(),
            category: None,
            save_path: None,
            content_layout: AddContentLayout::Original,
            file_priorities: None,
        };

        let outcome =
            tauri::async_runtime::block_on(async { client.add_torrents(&input).await.unwrap() });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(outcome.success_count, 0);
        assert_eq!(outcome.failure_count, 0);
        assert!(outcome.added_torrent_ids.is_empty());
        assert!(requests[0].starts_with("POST /api/v2/torrents/add "));
    }

    #[test]
    fn rejects_invalid_torrent_data_with_an_error() {
        let (endpoint, _requests, server) = serve_status_responses(vec![(415, "")]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let input = AddTorrentsInput {
            urls: Vec::new(),
            files: vec![AddTorrentFile {
                name: "broken.torrent".to_string(),
                base64_content: BASE64_STANDARD.encode(b"not-a-torrent"),
            }],
            category: None,
            save_path: None,
            content_layout: AddContentLayout::Original,
            file_priorities: None,
        };

        let outcome =
            tauri::async_runtime::block_on(async { client.add_torrents(&input).await });
        server.join().unwrap();

        assert!(outcome.is_err());
    }

    #[test]
    fn fetches_the_default_save_path() {
        let (endpoint, requests, server) = serve_responses(vec!["C:/Downloads"]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        let path = tauri::async_runtime::block_on(async { client.default_save_path().await.unwrap() });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(path, "C:/Downloads");
        assert!(requests[0].starts_with("GET /api/v2/app/defaultSavePath "));
    }

    #[test]
    fn parses_torrent_files_into_metadata() {
        let metadata_json = r#"[{
            "hash":"v2hash123",
            "infohash_v1":"v1hash123",
            "infohash_v2":"v2hash123",
            "info":{
                "name":"Show.S01",
                "files":[
                    {"path":"Show.S01/ep1.mkv","length":1000},
                    {"path":"Show.S01/extras/notes.txt","length":50}
                ]
            }
        }]"#;
        let (endpoint, requests, server) = serve_responses(vec![metadata_json]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let files = vec![AddTorrentFile {
            name: "show.torrent".to_string(),
            base64_content: BASE64_STANDARD.encode(b"parsed-torrent-bytes"),
        }];

        let metadata =
            tauri::async_runtime::block_on(async { client.parse_metadata(&files).await.unwrap() });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].hash, "v2hash123");
        assert_eq!(metadata[0].name, "Show.S01");
        assert_eq!(metadata[0].files.len(), 2);
        assert_eq!(metadata[0].files[0].path, "Show.S01/ep1.mkv");
        assert_eq!(metadata[0].files[0].length, 1000);
        assert!(requests[0].starts_with("POST /api/v2/torrents/parseMetadata "));
        assert!(requests[0].contains("filename=\"0\""));
        assert!(requests[0].contains("parsed-torrent-bytes"));
    }

    #[test]
    fn fetches_metadata_for_magnets_with_a_pending_phase() {
        let metadata_json = r#"{
            "hash":"v2hash123",
            "info":{"name":"Linux ISO","length":4096}
        }"#;
        let (endpoint, requests, server) =
            serve_status_responses(vec![(202, "{}"), (200, metadata_json)]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        let (pending, ready) = tauri::async_runtime::block_on(async {
            (
                client.fetch_metadata("magnet:?xt=urn:btih:abc").await.unwrap(),
                client.fetch_metadata("magnet:?xt=urn:btih:abc").await.unwrap(),
            )
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(matches!(pending, MetadataFetch::Pending));
        match ready {
            MetadataFetch::Ready { metadata } => {
                assert_eq!(metadata.hash, "v2hash123");
                assert_eq!(metadata.name, "Linux ISO");
                assert_eq!(metadata.files.len(), 1);
                assert_eq!(metadata.files[0].path, "Linux ISO");
                assert_eq!(metadata.files[0].length, 4096);
            }
            MetadataFetch::Pending => panic!("expected ready metadata"),
        }
        assert!(requests
            .iter()
            .all(|request| request.contains("source=magnet%3A%3Fxt%3Durn%3Abtih%3Aabc")));
    }

    #[test]
    fn adds_a_single_torrent_with_file_priorities() {
        let outcome_json =
            r#"{"success_count":1,"failure_count":0,"pending_count":0,"added_torrent_ids":["abc"]}"#;
        let (endpoint, requests, server) = serve_responses(vec![outcome_json]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();
        let input = AddTorrentsInput {
            urls: vec!["v2hash123".to_string()],
            files: Vec::new(),
            category: None,
            save_path: None,
            content_layout: AddContentLayout::Original,
            file_priorities: Some(vec![1, 0, 1]),
        };

        tauri::async_runtime::block_on(async { client.add_torrents(&input).await.unwrap() });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(requests[0].contains("name=\"filePriorities\""));
        assert!(requests[0].contains("1,0,1"));
    }

    #[test]
    fn fetches_properties_files_and_trackers_for_a_torrent() {
        let properties_json = r#"{
            "hash":"abc123",
            "name":"BigBuckBunny_124",
            "addition_date":1787950567,
            "completion_date":-1,
            "time_elapsed":28,
            "save_path":"C:/Downloads",
            "total_uploaded":0,
            "total_downloaded":76651643,
            "availability":4.17
        }"#;
        let files_json = r#"[
            {"index":0,"name":"BigBuckBunny_124/a.txt","size":10,"progress":1.0,"priority":1},
            {"name":"BigBuckBunny_124/b.bin","size":20,"progress":0.5,"priority":6}
        ]"#;
        // qBittorrent 5.2 reports per-endpoint announce state on real
        // trackers, plus synthetic DHT/PeX/LSD entries with tier -1.
        let trackers_json = r#"[
            {"url":"** [DHT] **","tier":-1,"status":2,"msg":"","num_seeds":2,"num_peers":0,"num_leeches":0},
            {"url":"http://bt1.archive.org:6969/announce","tier":0,"status":2,"msg":"","num_seeds":10,"num_peers":14,"num_leeches":4,"endpoints":[{"name":"192.168.1.67:23786","bt_version":1,"status":2,"num_seeds":10,"num_peers":14,"num_leeches":4,"num_downloaded":0}]}
        ]"#;
        let (endpoint, requests, server) = serve_responses(vec![
            properties_json,
            files_json,
            trackers_json,
        ]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        let (properties, files, trackers) = tauri::async_runtime::block_on(async {
            (
                client.properties("abc123").await.unwrap(),
                client.files("abc123").await.unwrap(),
                client.trackers("abc123").await.unwrap(),
            )
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(properties.id, "abc123");
        assert_eq!(properties.name, "BigBuckBunny_124");
        assert_eq!(properties.added_on, 1787950567);
        assert_eq!(properties.completed_on, None);
        assert_eq!(properties.time_active, 28);
        assert_eq!(properties.save_path, "C:/Downloads");
        assert_eq!(properties.uploaded_total, 0);
        assert_eq!(properties.downloaded_total, 76651643);
        assert!((properties.availability - 4.17).abs() < 1e-9);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].id, 0);
        assert_eq!(files[0].path, "BigBuckBunny_124/a.txt");
        assert_eq!(files[0].priority, 1);
        assert_eq!(files[1].id, 1);
        assert_eq!(files[1].priority, 6);

        assert_eq!(trackers.len(), 2);
        assert_eq!(trackers[0].url, "** [DHT] **");
        assert_eq!(trackers[0].tier, -1);
        assert_eq!(trackers[1].url, "http://bt1.archive.org:6969/announce");
        assert_eq!(trackers[1].status, 2);
        assert_eq!(trackers[1].seeds, 10);
        assert_eq!(trackers[1].peers, 14);
        assert_eq!(trackers[1].leeches, 4);

        assert!(requests[0].starts_with("GET /api/v2/torrents/properties?hash=abc123 "));
        assert!(requests[1].starts_with("GET /api/v2/torrents/files?hash=abc123 "));
        assert!(requests[2].starts_with("GET /api/v2/torrents/trackers?hash=abc123 "));
    }

    #[test]
    fn sets_file_priorities_one_request_per_file() {
        let (endpoint, requests, server) = serve_responses(vec!["Ok.", "Ok."]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        tauri::async_runtime::block_on(async {
            client.set_file_priority("abc123", 0, 0).await.unwrap();
            client.set_file_priority("abc123", 2, 6).await.unwrap();
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(requests
            .iter()
            .all(|request| request.starts_with("POST /api/v2/torrents/filePrio ")));
        assert!(requests[0].contains("hash=abc123&id=0&priority=0"));
        assert!(requests[1].contains("hash=abc123&id=2&priority=6"));
    }

    #[test]
    fn creates_a_category_before_assigning_it() {
        let (endpoint, requests, server) = serve_responses(vec!["Ok.", "Ok."]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        tauri::async_runtime::block_on(async {
            client
                .set_category_creating_missing(&["abc".to_string(), "def".to_string()], "Fresh")
                .await
                .unwrap();
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(requests[0].starts_with("POST /api/v2/torrents/createCategory "));
        assert!(requests[0].contains("category=Fresh"));
        assert!(requests[1].starts_with("POST /api/v2/torrents/setCategory "));
        assert!(requests[1].contains("hashes=abc%7Cdef&category=Fresh"));
    }

    #[test]
    fn adds_and_removes_tags() {
        let (endpoint, requests, server) = serve_responses(vec!["Ok.", "Ok."]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        tauri::async_runtime::block_on(async {
            client
                .add_tags(&["abc".to_string(), "def".to_string()], &["Movies".to_string(), "Shows".to_string()])
                .await
                .unwrap();
            client
                .remove_tags(&["abc".to_string()], &["Movies".to_string()])
                .await
                .unwrap();
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert!(requests[0].starts_with("POST /api/v2/torrents/addTags "));
        assert!(requests[0].contains("hashes=abc%7Cdef&tags=Movies%2CShows"));
        assert!(requests[1].starts_with("POST /api/v2/torrents/removeTags "));
        assert!(requests[1].contains("hashes=abc&tags=Movies"));
    }

    #[test]
    fn fetches_categories_and_tags_sorted() {
        let categories_json =
            r#"{"Shows":{"name":"Shows","savePath":""},"Movies":{"name":"Movies","savePath":""}}"#;
        let (endpoint, requests, server) =
            serve_responses(vec![categories_json, r#"["Shows","Movies"]"#]);
        let client = QbittorrentClient::new(ConnectionInput {
            endpoint,
            authentication_mode: AuthenticationMode::ApiKey,
            api_key: Some("qbt_0000000000000000000000000000".to_string()),
            username: None,
            password: None,
        })
        .unwrap();

        let (categories, tags) = tauri::async_runtime::block_on(async {
            (client.categories().await.unwrap(), client.tags().await.unwrap())
        });
        server.join().unwrap();
        let requests: Vec<_> = requests.iter().collect();

        assert_eq!(categories, vec!["Movies".to_string(), "Shows".to_string()]);
        assert_eq!(tags, vec!["Movies".to_string(), "Shows".to_string()]);
        assert!(requests[0].starts_with("GET /api/v2/torrents/categories "));
        assert!(requests[1].starts_with("GET /api/v2/torrents/tags "));
    }

    #[test]
    fn normalizes_file_priorities_and_tags() {
        let valid = normalize_file_priorities(vec![
            TorrentFilePriority { id: 2, priority: 6 },
            TorrentFilePriority { id: 0, priority: 0 },
            TorrentFilePriority { id: 2, priority: 1 },
        ])
        .unwrap();
        assert_eq!(
            valid,
            vec![
                TorrentFilePriority { id: 2, priority: 6 },
                TorrentFilePriority { id: 0, priority: 0 },
            ]
        );

        assert!(normalize_file_priorities(vec![TorrentFilePriority { id: 0, priority: 4 }]).is_err());
        assert!(normalize_file_priorities(Vec::new()).is_err());

        let tags = normalize_tag_list(vec![
            "Movies".to_string(),
            " Movies ".to_string(),
            String::new(),
            "Shows".to_string(),
        ]);
        assert_eq!(tags, vec!["Movies".to_string(), "Shows".to_string()]);
    }

    fn serve_responses(
        bodies: Vec<&'static str>,
    ) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        serve_status_responses(bodies.into_iter().map(|body| (200, body)).collect())
    }

    fn serve_status_responses(
        responses: Vec<(u16, &'static str)>,
    ) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (request_sender, requests) = mpsc::channel();
        let server = thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                request_sender
                    .send(read_full_request(&mut stream).unwrap())
                    .unwrap();

                let reason = StatusCode::from_u16(status)
                    .ok()
                    .and_then(|code| code.canonical_reason())
                    .unwrap_or("Unknown");
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });

        (endpoint, requests, server)
    }

    // Streaming request bodies (multipart) can arrive across TCP segments,
    // so read until the declared content length has been received.
    fn read_full_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut buffer = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let length = stream.read(&mut chunk)?;
            buffer.extend_from_slice(&chunk[..length]);
            let text = String::from_utf8_lossy(&buffer).to_string();
            if let Some(header_end) = text.find("\r\n\r\n") {
                let content_length = text
                    .lines()
                    .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
                    .and_then(|line| line.split_once(':'))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if buffer.len() >= header_end + 4 + content_length {
                    return Ok(text);
                }
            }
            if length == 0 {
                return Ok(text);
            }
        }
    }
}
