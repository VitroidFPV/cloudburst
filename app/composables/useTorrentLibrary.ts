import { invoke } from '@tauri-apps/api/core'
import type { ConnectionInput, ConnectionProfile, ConnectionSnapshot, ConnectionStatus, RestoreOutcome, Torrent, TorrentFilter, TorrentFilterId } from '~/types/torrent'
import { isUiDebugActive, uiDebugTorrents } from '~/utils/ui-debug'

const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error)

export const useTorrentLibrary = () => {
  const torrents = useState<Torrent[]>('torrent-library', () => [])
  const activeFilter = useState<TorrentFilterId>('torrent-filter', () => 'all')
  const activeCategory = useState('torrent-category', () => '')
  const connectionStatus = useState<ConnectionStatus>('connection-status', () => 'disconnected')
  const connectionError = useState('connection-error', () => '')
  const connectionEndpoint = useState('connection-endpoint', () => '')
  const connectionVersion = useState('connection-version', () => '')
  const savedProfile = useState<ConnectionProfile | null>('saved-connection-profile', () => null)
  const stale = useState('connection-stale', () => false)
  const refreshing = useState('connection-refreshing', () => false)

  if (isUiDebugActive()) torrents.value = uiDebugTorrents

  const filters = computed<TorrentFilter[]>(() => [
    { id: 'all', label: 'All torrents', icon: 'i-lucide-list-filter', count: torrents.value.length },
    { id: 'downloading', label: 'Downloading', icon: 'i-lucide-arrow-down-to-line', count: torrents.value.filter(torrent => torrent.status === 'downloading').length },
    { id: 'seeding', label: 'Seeding', icon: 'i-lucide-arrow-up-from-line', count: torrents.value.filter(torrent => torrent.status === 'seeding').length },
    { id: 'paused', label: 'Paused', icon: 'i-lucide-pause', count: torrents.value.filter(torrent => torrent.status === 'paused').length },
    { id: 'attention', label: 'Needs attention', icon: 'i-lucide-triangle-alert', count: torrents.value.filter(torrent => ['stalled', 'error'].includes(torrent.status)).length },
  ])

  const categories = computed(() => [...new Set(torrents.value.map(torrent => torrent.category).filter(Boolean))].sort())

  const visibleTorrents = computed(() => torrents.value.filter((torrent) => {
    const matchesCategory = !activeCategory.value || torrent.category === activeCategory.value
    const matchesFilter = activeFilter.value === 'all'
      || (activeFilter.value === 'attention' && ['stalled', 'error'].includes(torrent.status))
      || torrent.status === activeFilter.value

    return matchesCategory && matchesFilter
  }))

  const transferTotals = computed(() => torrents.value.reduce((totals, torrent) => ({
    down: totals.down + torrent.downSpeed,
    up: totals.up + torrent.upSpeed,
  }), { down: 0, up: 0 }))

  const applySnapshot = (snapshot: ConnectionSnapshot) => {
    torrents.value = snapshot.torrents
    connectionEndpoint.value = snapshot.endpoint
    connectionVersion.value = snapshot.version
    connectionStatus.value = 'connected'
    connectionError.value = ''
    stale.value = false
  }

  const connect = async (input: ConnectionInput) => {
    connectionStatus.value = 'connecting'
    connectionError.value = ''

    try {
      const snapshot = await invoke<ConnectionSnapshot>('connect_qbittorrent', { input })
      applySnapshot(snapshot)
      savedProfile.value = {
        endpoint: snapshot.endpoint,
        authenticationMode: input.authenticationMode,
        username: input.authenticationMode === 'credentials' ? input.username : undefined,
      }
      return true
    } catch (error) {
      connectionStatus.value = 'disconnected'
      connectionError.value = errorMessage(error)
      stale.value = torrents.value.length > 0
      return false
    }
  }

  const restoreSavedConnection = async () => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return false

    connectionStatus.value = 'connecting'
    connectionError.value = ''

    try {
      const outcome = await invoke<RestoreOutcome>('restore_saved_qbittorrent')
      savedProfile.value = outcome.profile

      if (outcome.snapshot) {
        applySnapshot(outcome.snapshot)
        return true
      }

      connectionStatus.value = 'disconnected'
      connectionError.value = outcome.error || ''
      connectionEndpoint.value = outcome.profile?.endpoint || ''
      stale.value = torrents.value.length > 0
      return false
    } catch (error) {
      connectionStatus.value = 'disconnected'
      connectionError.value = errorMessage(error)
      stale.value = torrents.value.length > 0
      return false
    }
  }

  const refresh = async () => {
    if (refreshing.value || connectionStatus.value === 'connecting') return false
    refreshing.value = true

    try {
      const snapshot = await invoke<ConnectionSnapshot>('refresh_qbittorrent')
      applySnapshot(snapshot)
      return true
    } catch (error) {
      connectionStatus.value = 'disconnected'
      connectionError.value = errorMessage(error)
      stale.value = torrents.value.length > 0
      return false
    } finally {
      refreshing.value = false
    }
  }

  const disconnect = async () => {
    try {
      await invoke('disconnect_qbittorrent')
      torrents.value = []
      connectionStatus.value = 'disconnected'
      connectionError.value = ''
      connectionEndpoint.value = ''
      connectionVersion.value = ''
      savedProfile.value = null
      stale.value = false
      return true
    } catch (error) {
      connectionError.value = errorMessage(error)
      return false
    }
  }

  const chooseFilter = (filter: TorrentFilterId) => {
    activeFilter.value = filter
    activeCategory.value = ''
  }

  const chooseCategory = (category: string) => {
    activeCategory.value = category
    activeFilter.value = 'all'
  }

  return {
    torrents,
    visibleTorrents,
    filters,
    categories,
    activeFilter,
    activeCategory,
    transferTotals,
    connectionStatus,
    connectionError,
    connectionEndpoint,
    connectionVersion,
    savedProfile,
    stale,
    refreshing,
    connect,
    restoreSavedConnection,
    refresh,
    disconnect,
    chooseFilter,
    chooseCategory,
  }
}
