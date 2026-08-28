import type { TorrentStatus } from '~/types/torrent'

export const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B'

  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)

  return `${(bytes / 1024 ** exponent).toFixed(exponent > 2 ? 2 : 1)} ${units[exponent]}`
}

export const formatSpeed = (bytes: number) => bytes === 0 ? '—' : `${formatBytes(bytes)}/s`

const addedOnFormatter = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' })
const addedOnFullFormatter = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' })

export const formatAddedOn = (timestamp: number) => {
  const elapsedSeconds = Math.max(0, Math.floor(Date.now() / 1000) - timestamp)
  if (elapsedSeconds < 60) return 'Just now'
  if (elapsedSeconds < 3_600) return `${Math.floor(elapsedSeconds / 60)}m ago`
  if (elapsedSeconds < 86_400) return `${Math.floor(elapsedSeconds / 3_600)}h ago`
  if (elapsedSeconds < 30 * 86_400) return `${Math.floor(elapsedSeconds / 86_400)}d ago`

  const date = new Date(timestamp * 1000)
  const monthDay = addedOnFormatter.format(date)
  return date.getFullYear() === new Date().getFullYear() ? monthDay : `${monthDay}, ${date.getFullYear()}`
}

export const formatAddedOnFull = (timestamp: number) => addedOnFullFormatter.format(new Date(timestamp * 1000))

export const formatEta = (seconds: number | null, status: TorrentStatus) => {
  if (status === 'paused') return 'Paused'
  if (status === 'checking') return 'Checking'
  if (status === 'stalled') return 'Waiting'
  if (status === 'error') return '—'
  if (seconds === null || seconds >= 8_640_000) return '∞'

  const days = Math.floor(seconds / 86_400)
  const hours = Math.floor(seconds % 86_400 / 3_600)
  const minutes = Math.floor(seconds % 3_600 / 60)
  const remainingSeconds = seconds % 60

  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  if (minutes > 0) return `${minutes}m ${remainingSeconds}s`
  return `${remainingSeconds}s`
}

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
