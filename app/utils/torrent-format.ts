import type { TorrentStatus } from '~/types/torrent'

export const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B'

  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)

  return `${(bytes / 1024 ** exponent).toFixed(exponent > 2 ? 2 : 1)} ${units[exponent]}`
}

export const formatSpeed = (bytes: number) => bytes === 0 ? '—' : `${formatBytes(bytes)}/s`

export const statusLabel: Record<TorrentStatus, string> = {
  downloading: 'Downloading',
  seeding: 'Seeding',
  paused: 'Paused',
  checking: 'Checking',
  stalled: 'Stalled',
  error: 'Error',
}

export const statusIcon: Record<TorrentStatus, string> = {
  downloading: 'i-lucide-arrow-down-to-line',
  seeding: 'i-lucide-arrow-up-from-line',
  paused: 'i-lucide-pause',
  checking: 'i-lucide-scan-line',
  stalled: 'i-lucide-clock-3',
  error: 'i-lucide-triangle-alert',
}
