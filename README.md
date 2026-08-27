# Cloudburst

Cloudburst is a focused interface for managing a qBittorrent 5.2+ instance. The repository currently contains the Nuxt application scaffold and an in-memory torrent module; it does not connect to qBittorrent or include the planned native shell yet.

## Development

```sh
pnpm install
pnpm dev
```

The application runs at `http://localhost:3000`.

Useful checks:

```sh
pnpm typecheck
pnpm generate
```

## Current structure

- `app/components/TorrentDashboard.vue` owns the selected application layout.
- `app/composables/useTorrentLibrary.ts` is the module seam for torrent state and actions.
- `app/data/placeholder-torrents.ts` supplies temporary in-memory data.
- `CONTEXT.md` and `docs/adr/` record the product language and architecture decisions.

The placeholder module is intentionally local and non-persistent. Its implementation can later be replaced with qBittorrent integration without changing the page layout.
