import { invoke } from '@tauri-apps/api/core'
import type { AddTorrentFile, AddTorrentsInput, AddTorrentsOutcome, ConnectionInput, ConnectionProfileList, ConnectionSnapshot, MetadataFetch, ResolveOutcome, TorrentFile, TorrentFilePriority, TorrentMetadata, TorrentProperties, TorrentTracker } from '~/types/torrent'

export interface QbittorrentAdapter {
  connect: (input: ConnectionInput) => Promise<ConnectionSnapshot>
  resolve: () => Promise<ResolveOutcome>
  connectSaved: (profileId: string) => Promise<ConnectionSnapshot>
  removeProfile: (profileId: string) => Promise<ConnectionProfileList>
  listProfiles: () => Promise<ConnectionProfileList>
  refresh: () => Promise<ConnectionSnapshot>
  setTorrentPaused: (torrentIds: string[], paused: boolean) => Promise<ConnectionSnapshot>
  removeTorrents: (torrentIds: string[], deleteFiles: boolean) => Promise<ConnectionSnapshot>
  addTorrents: (input: AddTorrentsInput) => Promise<AddTorrentsOutcome>
  defaultSavePath: () => Promise<string>
  parseTorrentMetadata: (files: AddTorrentFile[]) => Promise<TorrentMetadata[]>
  fetchTorrentMetadata: (source: string) => Promise<MetadataFetch>
  fetchTorrentProperties: (torrentId: string) => Promise<TorrentProperties>
  fetchTorrentFiles: (torrentId: string) => Promise<TorrentFile[]>
  fetchTorrentTrackers: (torrentId: string) => Promise<TorrentTracker[]>
  setTorrentFilePriorities: (torrentId: string, priorities: TorrentFilePriority[]) => Promise<void>
  setTorrentCategory: (torrentIds: string[], category: string) => Promise<void>
  addTorrentTags: (torrentIds: string[], tags: string[]) => Promise<void>
  removeTorrentTags: (torrentIds: string[], tags: string[]) => Promise<void>
  fetchCategories: () => Promise<string[]>
  fetchTags: () => Promise<string[]>
  disconnect: () => Promise<void>
}

export const tauriQbittorrentAdapter: QbittorrentAdapter = {
  connect: input => invoke<ConnectionSnapshot>('connect_qbittorrent', { input }),
  resolve: () => invoke<ResolveOutcome>('resolve_connection'),
  connectSaved: profileId => invoke<ConnectionSnapshot>('connect_saved_qbittorrent', { id: profileId }),
  removeProfile: profileId => invoke<ConnectionProfileList>('remove_connection_profile', { id: profileId }),
  listProfiles: () => invoke<ConnectionProfileList>('list_connection_profiles'),
  refresh: () => invoke<ConnectionSnapshot>('refresh_qbittorrent'),
  setTorrentPaused: (torrentIds, paused) => invoke<ConnectionSnapshot>('set_torrents_paused', { torrentIds, paused }),
  removeTorrents: (torrentIds, deleteFiles) => invoke<ConnectionSnapshot>('remove_torrents', { torrentIds, deleteFiles }),
  addTorrents: input => invoke<AddTorrentsOutcome>('add_torrents', { input }),
  defaultSavePath: () => invoke<string>('fetch_default_save_path'),
  parseTorrentMetadata: files => invoke<TorrentMetadata[]>('parse_torrent_metadata', { files }),
  fetchTorrentMetadata: source => invoke<MetadataFetch>('fetch_torrent_metadata', { source }),
  fetchTorrentProperties: torrentId => invoke<TorrentProperties>('fetch_torrent_properties', { torrentId }),
  fetchTorrentFiles: torrentId => invoke<TorrentFile[]>('fetch_torrent_files', { torrentId }),
  fetchTorrentTrackers: torrentId => invoke<TorrentTracker[]>('fetch_torrent_trackers', { torrentId }),
  setTorrentFilePriorities: (torrentId, priorities) => invoke('set_torrent_file_priorities', { torrentId, priorities }),
  setTorrentCategory: (torrentIds, category) => invoke('set_torrent_category', { torrentIds, category }),
  addTorrentTags: (torrentIds, tags) => invoke('add_torrent_tags', { torrentIds, tags }),
  removeTorrentTags: (torrentIds, tags) => invoke('remove_torrent_tags', { torrentIds, tags }),
  fetchCategories: () => invoke<string[]>('fetch_categories'),
  fetchTags: () => invoke<string[]>('fetch_tags'),
  disconnect: () => invoke('disconnect_qbittorrent'),
}
