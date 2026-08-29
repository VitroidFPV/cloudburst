export type TorrentStatus = 'downloading' | 'seeding' | 'paused' | 'checking' | 'stalled' | 'error'

export type TorrentFilterId = 'all' | 'downloading' | 'seeding' | 'paused' | 'attention'

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected'

export type AuthenticationMode = 'apiKey' | 'credentials'

export interface Torrent {
  id: string
  name: string
  status: TorrentStatus
  progress: number
  size: number
  downloaded: number
  downSpeed: number
  upSpeed: number
  etaSeconds: number | null
  ratio: number
  seeds: number
  peers: number
  category: string
  tags: string[]
  addedOn: number
  savePath: string
}

export interface ConnectionInput {
  endpoint: string
  authenticationMode: AuthenticationMode
  apiKey?: string
  username?: string
  password?: string
}

export interface ConnectionProfile {
  endpoint: string
  authenticationMode: AuthenticationMode
  username?: string
}

export interface ConnectionSnapshot {
  endpoint: string
  version: string
  torrents: Torrent[]
}

export interface RestoreOutcome {
  profile: ConnectionProfile | null
  snapshot: ConnectionSnapshot | null
  error: string | null
}

export type AddContentLayout = 'original' | 'subfolder' | 'noSubfolder'

export interface AddTorrentFile {
  name: string
  base64Content: string
}

export interface AddTorrentsInput {
  urls: string[]
  files: AddTorrentFile[]
  category?: string
  savePath?: string
  contentLayout: AddContentLayout
  filePriorities?: number[]
}

export interface AddTorrentsOutcome {
  successCount: number
  failureCount: number
  pendingCount: number
  addedTorrentIds: string[]
}

export interface TorrentMetadataFile {
  path: string
  length: number
}

export interface TorrentMetadata {
  hash: string
  name: string
  files: TorrentMetadataFile[]
}

export type MetadataFetch = { status: 'ready', metadata: TorrentMetadata } | { status: 'pending' }

export type TorrentFilePriorityValue = 0 | 1 | 6 | 7

export interface TorrentProperties {
  id: string
  name: string
  addedOn: number
  completedOn: number | null
  timeActive: number
  savePath: string
  uploadedTotal: number
  downloadedTotal: number
  availability: number
}

export interface TorrentFile {
  id: number
  path: string
  size: number
  progress: number
  priority: number
}

export interface TorrentTracker {
  url: string
  tier: number
  status: number
  message: string
  seeds: number
  peers: number
  leeches: number
}

export interface TorrentFilePriority {
  id: number
  priority: TorrentFilePriorityValue
}

export type MagnetHandlerStatus = 'cloudburstDefault' | 'otherProgram' | 'notRegistered'

export interface TorrentFilter {
  id: TorrentFilterId
  label: string
  icon: string
  count: number
}
