# Cloudburst

Cloudburst is a focused interface for managing a qBittorrent 5.2+ instance. The repository currently contains a Nuxt interface and a Tauri 2 desktop shell with persistent tray behavior and a read/write qBittorrent Web API connection.

## Development

```sh
pnpm install
pnpm dev
```

The application runs at `http://localhost:3000`. The browser build can render the interface, but connecting to qBittorrent requires the desktop shell.

## Desktop development

Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system. The repository pins Rust 1.88 and rustup will install that toolchain when needed. Then run:

```sh
pnpm dev:desktop
```

Tauri starts the Nuxt development server automatically and keeps Nuxt hot-module replacement active inside the desktop window. Changes to the Rust shell also trigger a native rebuild. To compile the desktop application and platform bundles:

```sh
pnpm build:desktop
```

The desktop build runs `pnpm generate` before packaging the generated client application.

Useful checks:

```sh
pnpm lint
pnpm test
pnpm typecheck
pnpm generate
```

## Current structure

- `app/components/TorrentDashboard.vue` owns the application layout and connection orchestration.
- `app/components/TorrentTable.vue` owns torrent selection, the row context menu, torrent activity and removal controls, columns, sorting, resizing, and persisted table preferences.
- `app/composables/useTorrentLibrary.ts` is the typed frontend seam for connection and torrent state.
- `src-tauri/src/qbittorrent.rs` owns qBittorrent authentication, compatibility checks, and Web API access.
- `src-tauri/` contains the desktop application shell and its permissions.
- `Cloudburst.svg` is the source asset for the browser favicon and generated desktop icons.
- `CONTEXT.md` and `docs/adr/` record the product language and architecture decisions.

Cloudburst saves one active connection profile and restores it when the desktop application starts. API keys and passwords are kept in Windows Credential Manager or the Linux Secret Service; the profile file contains only the WebUI URL, authentication mode, and optional username. Plain HTTP is accepted only for loopback addresses; remote qBittorrent connections must use HTTPS. Cloudburst requires qBittorrent 5.2 or newer, can start, stop, and remove selected torrents from the table toolbar or row context menu — removing a torrent confirms first and keeps the downloaded content unless "Remove and files" is chosen — and retains the last known torrents as stale while reconnecting with capped backoff if the active connection is lost. Torrents are added through a two-step dialog: sources first (magnet links or .torrent URLs one per line, .torrent files via picker or window-wide drag-and-drop), then a review step with category, folder layout, and save location — where a native folder picker appears for local instances — plus a toggleable file tree for single-source adds that lets the user keep or skip individual files before submitting; magnets resolve their file list through qBittorrent's metadata fetch while the dialog stays open. `magnet:` links clicked in the browser open the dialog prefilled — Cloudburst registers the scheme at startup, advertises itself to Windows as a magnet handler (so browsers and the Settings app can offer it), always opens the dialog for review before submitting, and warns with a shortcut to Windows Settings if the system still routes magnet links to another program; the instance reports successes, rejections (usually duplicates), and sources still being fetched. When the active instance is local, the add dialog offers a native folder picker for the save location. Multiple profiles and process-based connection resolution are not implemented yet.

Closing the desktop window hides Cloudburst in the system tray; use the tray icon or **Show Cloudburst** to restore it, and **Quit Cloudburst** to exit the process.
