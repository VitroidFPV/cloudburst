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

## Releases

Pull requests and pushes to `main` run the frontend lint, typecheck, tests, static build, and Rust tests in GitHub Actions. Pull requests also produce unsigned Windows, macOS, and Linux preview packages; download them from the **Artifacts** section of the PR's `PR preview builds` workflow run.

Releases use semantic versions and Git tags. Prepare a release with:

```sh
pnpm release:prepare 0.2.0
pnpm install --lockfile-only
git add package.json pnpm-lock.yaml src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "Release: 0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```

The tag starts builds for Windows, macOS, and Linux and creates a draft GitHub Release with generated notes and the platform installers attached. Review and publish the draft in GitHub. The generated packages are unsigned; configure the platform signing secrets before presenting them as trusted production downloads.

## Current structure

- `app/components/TorrentDashboard.vue` owns the application layout and connection orchestration.
- `app/components/TorrentTable.vue` owns torrent selection, the row context menu, torrent activity and removal controls, columns, sorting, resizing, and persisted table preferences.
- `app/composables/useTorrentLibrary.ts` is the typed frontend seam for connection and torrent state.
- `src-tauri/src/qbittorrent.rs` owns qBittorrent authentication, compatibility checks, and Web API access.
- `src-tauri/` contains the desktop application shell and its permissions.
- `Cloudburst.svg` is the source asset for the browser favicon and generated desktop icons.
- `CONTEXT.md` and `docs/adr/` record the product language and architecture decisions.

## Behavior

### Connections

Cloudburst retains multiple connection profiles and performs connection resolution when the desktop application starts: the last active profile is attempted first, then the remaining saved profiles, and the first reachable instance wins. The profile store contains only each WebUI URL, authentication mode, optional username, and the id of the last connected profile; credentials are kept in Windows Credential Manager or the Linux Secret Service. Going offline keeps every saved profile, and a profile's Connect and Forget actions live in the connection settings.

Plain HTTP is accepted only for loopback addresses; remote qBittorrent connections must use HTTPS. Cloudburst requires qBittorrent 5.2 or newer.

### Torrent management

Torrents can be started, stopped, and removed from the table toolbar or the row context menu. Removing a torrent confirms first and keeps the downloaded content unless "Remove and files" is chosen. When the active connection is lost, the last known torrents stay visible as stale while Cloudburst reconnects with capped backoff.

### Adding torrents

Torrents are added through a two-step dialog: sources first, then a review step. Sources are magnet links or .torrent URLs entered one per line, and .torrent files picked from disk or dropped anywhere on the window. The review step offers a category, folder layout, and save location — with a native folder picker for local instances — plus a toggleable file tree for single-source adds that lets the user keep or skip individual files before submitting. Magnets resolve their file list through qBittorrent's metadata fetch while the dialog stays open.

The instance reports successes, rejections (usually duplicates), and sources still being fetched.

### Magnet links

`magnet:` links clicked in the browser open the dialog prefilled. Cloudburst registers the scheme at startup and advertises itself to Windows as a magnet handler, so browsers and the Settings app can offer it. The dialog always opens for review before submitting, and Cloudburst warns with a shortcut to Windows Settings if the system still routes magnet links to another program.

### System tray

Closing the desktop window hides Cloudburst in the system tray; use the tray icon or **Show Cloudburst** to restore it, and **Quit Cloudburst** to exit the process.

### Notifications

Native torrent notifications can be enabled in the application settings. Cloudburst notifies when a known torrent finishes downloading or enters an error state; loading an existing library does not produce notifications.
