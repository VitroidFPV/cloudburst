import type { AddTorrentFile, AddTorrentsInput, AddTorrentsOutcome, ConnectionInput, ConnectionProfile, ConnectionSnapshot, ConnectionStatus, MetadataFetch, RestoreOutcome, Torrent, TorrentFile, TorrentFilePriority, TorrentFilter, TorrentFilterId, TorrentMetadata, TorrentProperties, TorrentTracker } from '~/types/torrent'
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
  const defaultSavePath = useState('default-save-path', () => '')

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

  const removeTorrents = async (torrentIds: string[], deleteFiles: boolean) => {
    const uniqueTorrentIds = [...new Set(torrentIds.map(id => id.trim()).filter(Boolean))]
    if (!uniqueTorrentIds.length || connectionStatus.value !== 'connected' || stale.value) return false

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      const snapshot = await adapter.removeTorrents(uniqueTorrentIds, deleteFiles)
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

  const addTorrents = async (input: AddTorrentsInput): Promise<AddTorrentsOutcome | null> => {
    const hasSources = input.urls.some(url => url.trim()) || input.files.length > 0
    if (!hasSources || connectionStatus.value !== 'connected' || stale.value) return null

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      const outcome = await adapter.addTorrents(input)
      if (!isCurrentOperation(operation)) return null

      // The add endpoint reports counts instead of a snapshot; adopt the
      // authoritative list once the instance has processed the request.
      const snapshot = await adapter.refresh()
      if (isCurrentOperation(operation)) applySnapshot(snapshot)
      return outcome
    }
    catch (error) {
      if (!isCurrentOperation(operation)) return null
      torrentActionError.value = errorMessage(error)
      return null
    }
    finally {
      if (isCurrentOperation(operation)) activityUpdating.value = false
    }
  }

  const loadDefaultSavePath = async () => {
    if (defaultSavePath.value || connectionStatus.value !== 'connected' || stale.value) return

    try {
      defaultSavePath.value = await adapter.defaultSavePath()
    }
    catch {
      defaultSavePath.value = ''
    }
  }

  const parseTorrentMetadata = async (files: AddTorrentFile[]): Promise<TorrentMetadata[] | null> => {
    const cleaned = files.filter(file => file.name.trim() && file.base64Content.trim())
    if (!cleaned.length || connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.parseTorrentMetadata(cleaned)
    }
    catch {
      return null
    }
  }

  const fetchTorrentMetadata = async (source: string): Promise<MetadataFetch | null> => {
    const trimmed = source.trim()
    if (!trimmed || connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.fetchTorrentMetadata(trimmed)
    }
    catch {
      return null
    }
  }

  // Detail-panel pollers run on their own cadence and must not interfere
  // with library operations, so they stay outside the operation generation.
  const fetchTorrentProperties = async (torrentId: string): Promise<TorrentProperties | null> => {
    if (connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.fetchTorrentProperties(torrentId)
    }
    catch {
      return null
    }
  }

  const fetchTorrentFiles = async (torrentId: string): Promise<TorrentFile[] | null> => {
    if (connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.fetchTorrentFiles(torrentId)
    }
    catch {
      return null
    }
  }

  const fetchTorrentTrackers = async (torrentId: string): Promise<TorrentTracker[] | null> => {
    if (connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.fetchTorrentTrackers(torrentId)
    }
    catch {
      return null
    }
  }

  const setTorrentFilePriorities = async (torrentId: string, priorities: TorrentFilePriority[]) => {
    if (!priorities.length || connectionStatus.value !== 'connected' || stale.value) return false

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      await adapter.setTorrentFilePriorities(torrentId, priorities)
      if (!isCurrentOperation(operation)) return false

      const snapshot = await adapter.refresh()
      if (isCurrentOperation(operation)) applySnapshot(snapshot)
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

  const setTorrentCategory = async (torrentIds: string[], category: string) => {
    const uniqueTorrentIds = [...new Set(torrentIds.map(id => id.trim()).filter(Boolean))]
    if (!uniqueTorrentIds.length || connectionStatus.value !== 'connected' || stale.value) return false

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      await adapter.setTorrentCategory(uniqueTorrentIds, category)
      if (!isCurrentOperation(operation)) return false

      const snapshot = await adapter.refresh()
      if (isCurrentOperation(operation)) applySnapshot(snapshot)
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

  const addTorrentTags = async (torrentIds: string[], tags: string[]) => {
    const uniqueTorrentIds = [...new Set(torrentIds.map(id => id.trim()).filter(Boolean))]
    if (!uniqueTorrentIds.length || connectionStatus.value !== 'connected' || stale.value) return false

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      await adapter.addTorrentTags(uniqueTorrentIds, tags)
      if (!isCurrentOperation(operation)) return false

      const snapshot = await adapter.refresh()
      if (isCurrentOperation(operation)) applySnapshot(snapshot)
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

  const removeTorrentTags = async (torrentIds: string[], tags: string[]) => {
    const uniqueTorrentIds = [...new Set(torrentIds.map(id => id.trim()).filter(Boolean))]
    if (!uniqueTorrentIds.length || connectionStatus.value !== 'connected' || stale.value) return false

    const operation = beginOperation()
    activityUpdating.value = true
    torrentActionError.value = ''

    try {
      await adapter.removeTorrentTags(uniqueTorrentIds, tags)
      if (!isCurrentOperation(operation)) return false

      const snapshot = await adapter.refresh()
      if (isCurrentOperation(operation)) applySnapshot(snapshot)
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

  const fetchCategories = async (): Promise<string[] | null> => {
    if (connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.fetchCategories()
    }
    catch {
      return null
    }
  }

  const fetchTags = async (): Promise<string[] | null> => {
    if (connectionStatus.value !== 'connected' || stale.value) return null

    try {
      return await adapter.fetchTags()
    }
    catch {
      return null
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
      defaultSavePath.value = ''
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
    removeTorrents,
    addTorrents,
    loadDefaultSavePath,
    defaultSavePath,
    parseTorrentMetadata,
    fetchTorrentMetadata,
    fetchTorrentProperties,
    fetchTorrentFiles,
    fetchTorrentTrackers,
    setTorrentFilePriorities,
    setTorrentCategory,
    addTorrentTags,
    removeTorrentTags,
    fetchCategories,
    fetchTags,
    retry,
    startAutoRefresh,
    disconnect,
    chooseFilter,
    chooseCategory,
  }
}
