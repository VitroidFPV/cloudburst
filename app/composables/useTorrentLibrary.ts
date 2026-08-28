import type { ConnectionInput, ConnectionProfile, ConnectionSnapshot, ConnectionStatus, RestoreOutcome, Torrent, TorrentFilter, TorrentFilterId } from '~/types/torrent'
import { tauriQbittorrentAdapter, type QbittorrentAdapter } from '~/adapters/qbittorrent'
import { isUiDebugActive, uiDebugTorrents } from '~/utils/ui-debug'

const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error)

export const CONNECTION_POLL_INTERVAL_MS = 5_000
export const CONNECTION_RETRY_DELAYS_MS = [1_000, 2_000, 5_000, 10_000, 30_000] as const

export const useTorrentLibrary = (adapter: QbittorrentAdapter = tauriQbittorrentAdapter) => {
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
  const activityUpdating = useState('torrent-activity-updating', () => false)
  const torrentActionError = useState('torrent-action-error', () => '')
  const operationGeneration = useState('connection-operation-generation', () => 0)

  if (adapter === tauriQbittorrentAdapter && isUiDebugActive()) torrents.value = uiDebugTorrents

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

  const beginOperation = () => {
    operationGeneration.value += 1
    refreshing.value = false
    activityUpdating.value = false
    return operationGeneration.value
  }

  const isCurrentOperation = (operation: number) => operationGeneration.value === operation

  const applySnapshot = (snapshot: ConnectionSnapshot) => {
    torrents.value = snapshot.torrents
    connectionEndpoint.value = snapshot.endpoint
    connectionVersion.value = snapshot.version
    connectionStatus.value = 'connected'
    connectionError.value = ''
    stale.value = false
  }

  const applyFailure = (operation: number, error: unknown) => {
    if (!isCurrentOperation(operation)) return false

    connectionStatus.value = 'disconnected'
    connectionError.value = errorMessage(error)
    stale.value = torrents.value.length > 0
    return false
  }

  const connect = async (input: ConnectionInput) => {
    const operation = beginOperation()
    connectionStatus.value = 'connecting'
    connectionError.value = ''

    try {
      const snapshot = await adapter.connect(input)
      if (!isCurrentOperation(operation)) return false

      applySnapshot(snapshot)
      savedProfile.value = {
        endpoint: snapshot.endpoint,
        authenticationMode: input.authenticationMode,
        username: input.authenticationMode === 'credentials' ? input.username : undefined,
      }
      return true
    }
    catch (error) {
      return applyFailure(operation, error)
    }
  }

  const restoreSavedConnection = async () => {
    if (adapter === tauriQbittorrentAdapter && (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window))) return false

    const operation = beginOperation()
    connectionStatus.value = 'connecting'
    connectionError.value = ''

    try {
      const outcome: RestoreOutcome = await adapter.restore()
      if (!isCurrentOperation(operation)) return false

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
    }
    catch (error) {
      return applyFailure(operation, error)
    }
  }

  const refresh = async () => {
    if (refreshing.value || activityUpdating.value || connectionStatus.value === 'connecting') return

    const operation = beginOperation()
    refreshing.value = true

    try {
      const snapshot = await adapter.refresh()
      if (!isCurrentOperation(operation)) return false

      applySnapshot(snapshot)
      return true
    }
    catch (error) {
      return applyFailure(operation, error)
    }
    finally {
      if (isCurrentOperation(operation)) refreshing.value = false
    }
  }

  const setTorrentsPaused = async (torrentIds: string[], paused: boolean) => {
    const uniqueTorrentIds = [...new Set(torrentIds.map(id => id.trim()).filter(Boolean))]
    if (!uniqueTorrentIds.length || connectionStatus.value !== 'connected' || stale.value) return false

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      const snapshot = await adapter.setTorrentPaused(uniqueTorrentIds, paused)
      if (!isCurrentOperation(operation)) return false

      applySnapshot(snapshot)
      return true
    }
    catch (error) {
      if (!isCurrentOperation(operation)) return false
      torrentActionError.value = errorMessage(error)
      return false
    }
    finally {
      if (isCurrentOperation(operation)) activityUpdating.value = false
    }
  }

  const disconnect = async () => {
    const operation = beginOperation()

    try {
      await adapter.disconnect()
      if (!isCurrentOperation(operation)) return false

      torrents.value = []
      connectionStatus.value = 'disconnected'
      connectionError.value = ''
      connectionEndpoint.value = ''
      connectionVersion.value = ''
      savedProfile.value = null
      stale.value = false
      torrentActionError.value = ''
      return true
    }
    catch (error) {
      if (!isCurrentOperation(operation)) return false
      connectionError.value = errorMessage(error)
      return false
    }
  }

  const retry = () => connectionStatus.value === 'disconnected' && savedProfile.value
    ? restoreSavedConnection()
    : refresh()

  const startAutoRefresh = () => {
    let stopped = false
    let retryIndex = 0
    let timer: ReturnType<typeof setTimeout> | undefined

    const schedule = (delay: number) => {
      if (stopped) return
      timer = setTimeout(() => void poll(), delay)
    }

    const poll = async () => {
      if (stopped) return

      let successful: boolean | undefined
      if (connectionStatus.value === 'connected') successful = await refresh()
      else if (savedProfile.value) successful = await restoreSavedConnection()

      if (stopped) return
      if (successful === false && savedProfile.value) {
        const delay = CONNECTION_RETRY_DELAYS_MS[Math.min(retryIndex, CONNECTION_RETRY_DELAYS_MS.length - 1)]
        retryIndex = Math.min(retryIndex + 1, CONNECTION_RETRY_DELAYS_MS.length - 1)
        schedule(delay ?? CONNECTION_POLL_INTERVAL_MS)
        return
      }

      retryIndex = 0
      schedule(CONNECTION_POLL_INTERVAL_MS)
    }

    schedule(CONNECTION_POLL_INTERVAL_MS)

    return () => {
      stopped = true
      if (timer) clearTimeout(timer)
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
    activityUpdating,
    torrentActionError,
    connect,
    restoreSavedConnection,
    refresh,
    setTorrentsPaused,
    retry,
    startAutoRefresh,
    disconnect,
    chooseFilter,
    chooseCategory,
  }
}
