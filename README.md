# Cloudburst

Cloudburst is a focused interface for managing a qBittorrent 5.2+ instance. The repository currently contains the Nuxt application scaffold, an in-memory torrent module, and a minimal Tauri 2 desktop shell. It does not connect to qBittorrent or implement native features yet.

## Development

```sh
pnpm install
pnpm dev
```

The application runs at `http://localhost:3000`.

## Desktop development

Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system. The repository pins Rust 1.88 and rustup will install that toolchain when needed. Then run:

```sh
pnpm tauri dev
```

Tauri starts the Nuxt development server automatically. To compile the desktop application and platform bundles:

```sh
pnpm tauri build
```

The desktop build runs `pnpm generate` before packaging the generated client application.

Useful checks:

```sh
pnpm typecheck
pnpm generate
```

## Current structure

- `app/components/TorrentDashboard.vue` owns the selected application layout.
- `app/composables/useTorrentLibrary.ts` is the module seam for torrent state and actions.
- `app/data/placeholder-torrents.ts` supplies temporary in-memory data.
- `src-tauri/` contains the minimal desktop application shell and its permissions.
- `Cloudburst.svg` is the source asset for the browser favicon and generated desktop icons.
- `CONTEXT.md` and `docs/adr/` record the product language and architecture decisions.

The placeholder module is intentionally local and non-persistent. Its implementation can later be replaced with qBittorrent integration without changing the page layout. The Tauri shell currently exposes no Cloudburst commands, tray behavior, or qBittorrent functionality.
