use crate::connection_profile::{self, AuthenticationMode, ConnectionProfile, StoredConnection};
use reqwest::{Client, RequestBuilder, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::RwLock, time::Duration};
use tauri::State;

const MINIMUM_VERSION: (u64, u64, u64) = (5, 2, 0);

#[derive(Default)]
pub struct ConnectionManager {
    active: RwLock<Option<ActiveConnection>>,
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
    let (input, profile, secret) = resolve_connection_input(input, &app).await?;
    let (active, snapshot) = establish_connection(input)
        .await
        .map_err(|error| error.to_string())?;
    connection_profile::save(&app, profile, secret).await?;

    set_active_connection(&manager, active)?;

    Ok(snapshot)
}

#[tauri::command]
pub async fn restore_saved_qbittorrent(
    manager: State<'_, ConnectionManager>,
    app: tauri::AppHandle,
) -> Result<RestoreOutcome, String> {
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
            Ok((active, snapshot)) => match set_active_connection(&manager, active) {
                Ok(()) => RestoreOutcome {
                    profile: Some(profile),
                    snapshot: Some(snapshot),
                    error: None,
                },
                Err(error) => RestoreOutcome {
                    profile: Some(profile),
                    snapshot: None,
                    error: Some(error),
                },
            },
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
    let active = manager
        .active
        .read()
        .map_err(|_| "The qBittorrent connection state is unavailable.".to_string())?
        .clone()
        .ok_or_else(|| "No qBittorrent connection is configured.".to_string())?;

    active.snapshot().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn disconnect_qbittorrent(
    manager: State<'_, ConnectionManager>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    connection_profile::clear(&app).await?;
    let mut connection = manager
        .active
        .write()
        .map_err(|_| "The qBittorrent connection state is unavailable.".to_string())?;
    *connection = None;
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

fn set_active_connection(
    manager: &ConnectionManager,
    active: ActiveConnection,
) -> Result<(), String> {
    let mut connection = manager
        .active
        .write()
        .map_err(|_| "The qBittorrent connection state is unavailable.".to_string())?;
    *connection = Some(active);
    Ok(())
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

    async fn get(&self, path: &str) -> Result<reqwest::Response, ConnectionError> {
        let url = self.endpoint.join(path).map_err(|error| {
            ConnectionError::InvalidConfiguration(format!(
                "Failed to build a qBittorrent API URL: {error}"
            ))
        })?;
        let request = self.authentication.apply(self.http.get(url));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
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

    fn serve_responses(
        bodies: Vec<&'static str>,
    ) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (request_sender, requests) = mpsc::channel();
        let server = thread::spawn(move || {
            for body in bodies {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let length = stream.read(&mut request).unwrap();
                request_sender
                    .send(String::from_utf8_lossy(&request[..length]).to_string())
                    .unwrap();

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });

        (endpoint, requests, server)
    }
}
