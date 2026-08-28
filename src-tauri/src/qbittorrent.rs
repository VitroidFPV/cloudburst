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
            // 409: nothing was added, usually duplicates. All carry the outcome body.
            status if status.is_success() || status == StatusCode::CONFLICT => {
                let text = response.text().await.map_err(|error| {
                    ConnectionError::InvalidResponse(format!(
                        "qBittorrent returned an unreadable add response: {error}"
                    ))
                })?;
                if text.trim().is_empty() {
                    return Ok(AddTorrentsOutcome::default());
                }
                serde_json::from_str::<QbittorrentAddOutcome>(&text)
                    .map(Into::into)
                    .map_err(|error| {
                        ConnectionError::InvalidResponse(format!(
                            "qBittorrent returned an unreadable add response: {error}"
                        ))
                    })
            }
            status => Err(ConnectionError::ConnectionFailed(format!(
                "qBittorrent at {} returned HTTP {status}.",
                self.display_endpoint()
            ))),
        }
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response, ConnectionError> {
        let url = self.api_url(path)?;
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
