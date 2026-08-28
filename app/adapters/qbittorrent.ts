import { invoke } from '@tauri-apps/api/core'
import type { AddTorrentsInput, AddTorrentsOutcome, ConnectionInput, ConnectionSnapshot, RestoreOutcome } from '~/types/torrent'

export interface QbittorrentAdapter {
  connect: (input: ConnectionInput) => Promise<ConnectionSnapshot>
  restore: () => Promise<RestoreOutcome>
  refresh: () => Promise<ConnectionSnapshot>
  setTorrentPaused: (torrentIds: string[], paused: boolean) => Promise<ConnectionSnapshot>
  removeTorrents: (torrentIds: string[], deleteFiles: boolean) => Promise<ConnectionSnapshot>
  addTorrents: (input: AddTorrentsInput) => Promise<AddTorrentsOutcome>
  defaultSavePath: () => Promise<string>
  disconnect: () => Promise<void>
}

export const tauriQbittorrentAdapter: QbittorrentAdapter = {
  connect: input => invoke<ConnectionSnapshot>('connect_qbittorrent', { input }),
  restore: () => invoke<RestoreOutcome>('restore_saved_qbittorrent'),
  refresh: () => invoke<ConnectionSnapshot>('refresh_qbittorrent'),
  setTorrentPaused: (torrentIds, paused) => invoke<ConnectionSnapshot>('set_torrents_paused', { torrentIds, paused }),
  removeTorrents: (torrentIds, deleteFiles) => invoke<ConnectionSnapshot>('remove_torrents', { torrentIds, deleteFiles }),
  addTorrents: input => invoke<AddTorrentsOutcome>('add_torrents', { input }),
  defaultSavePath: () => invoke<string>('fetch_default_save_path'),
  disconnect: () => invoke('disconnect_qbittorrent'),
}
