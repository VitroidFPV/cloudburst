export type TorrentStatus = 'downloading' | 'seeding' | 'paused' | 'checking' | 'stalled' | 'error'

export type TorrentFilterId = 'all' | 'downloading' | 'seeding' | 'paused' | 'attention'

export type TorrentAction = 'resume' | 'pause' | 'remove'

export interface Torrent {
  id: string
  name: string
  status: TorrentStatus
  progress: number
  size: number
  downloaded: number
  downSpeed: number
  upSpeed: number
  eta: string
  ratio: number
  seeds: number
  peers: number
  category: string
  tags: string[]
  added: string
  savePath: string
}

export interface TorrentFilter {
  id: TorrentFilterId
  label: string
  icon: string
  count: number
}

export interface ActionFeedback {
  title: string
  description: string
}
