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
  apiKey?: string
  username?: string
  password?: string
}

export interface ConnectionSnapshot {
  endpoint: string
  version: string
  torrents: Torrent[]
}

export interface TorrentFilter {
  id: TorrentFilterId
  label: string
  icon: string
  count: number
}
