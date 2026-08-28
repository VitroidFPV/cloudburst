import { invoke } from '@tauri-apps/api/core'
import type { ConnectionInput, ConnectionSnapshot, RestoreOutcome } from '~/types/torrent'

export interface QbittorrentAdapter {
  connect: (input: ConnectionInput) => Promise<ConnectionSnapshot>
  restore: () => Promise<RestoreOutcome>
  refresh: () => Promise<ConnectionSnapshot>
  disconnect: () => Promise<void>
}

export const tauriQbittorrentAdapter: QbittorrentAdapter = {
  connect: input => invoke<ConnectionSnapshot>('connect_qbittorrent', { input }),
  restore: () => invoke<RestoreOutcome>('restore_saved_qbittorrent'),
  refresh: () => invoke<ConnectionSnapshot>('refresh_qbittorrent'),
  disconnect: () => invoke('disconnect_qbittorrent'),
}
