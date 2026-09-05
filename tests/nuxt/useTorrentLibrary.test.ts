import { clearNuxtState } from '#app'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import type { QbittorrentAdapter } from '~/adapters/qbittorrent'
import { usePlaceholderSetting } from '~/composables/usePlaceholderSetting'
import { CONNECTION_POLL_INTERVAL_MS, CONNECTION_RETRY_DELAYS_MS, useTorrentLibrary } from '~/composables/useTorrentLibrary'
import type { AddTorrentsInput, AddTorrentsOutcome, ConnectionInput, ConnectionProfile, ConnectionProfileList, ConnectionSnapshot, ResolveOutcome, Torrent } from '~/types/torrent'

const connectionInput: ConnectionInput = {
  endpoint: 'http://localhost:8080',
  authenticationMode: 'apiKey',
  apiKey: 'qbt_0000000000000000000000000000',
}

const profile: ConnectionProfile = {
  id: 'http://localhost:8080|ApiKey|',
  endpoint: 'http://localhost:8080',
  authenticationMode: 'apiKey',
}

const remoteProfile: ConnectionProfile = {
  id: 'http://nas.lan:8080|ApiKey|',
  endpoint: 'http://nas.lan:8080',
  authenticationMode: 'apiKey',
}

const torrent: Torrent = {
  id: 'torrent-1',
  name: 'Debian ISO',
  status: 'downloading',
  progress: 62.5,
  size: 4096,
  downloaded: 2560,
  downSpeed: 1024,
  upSpeed: 128,
  etaSeconds: 90,
  ratio: 0.5,
  seeds: 12,
  peers: 3,
  category: 'Linux',
  tags: ['iso'],
  addedOn: 1_700_000_000,
  savePath: 'C:/Downloads',
}

const snapshot: ConnectionSnapshot = {
  endpoint: profile.endpoint,
  version: '5.2.1',
  torrents: [torrent],
}

const remoteSnapshot: ConnectionSnapshot = {
  endpoint: remoteProfile.endpoint,
  version: '5.2.1',
  torrents: [torrent],
}

const profileList = (profiles: ConnectionProfile[], activeId: string | null = null): ConnectionProfileList => ({
  profiles,
  activeId,
})

const unexpected = async (): Promise<never> => {
  throw new Error('Unexpected adapter call')
}

const createAdapter = (overrides: Partial<QbittorrentAdapter> = {}): QbittorrentAdapter => ({
  connect: unexpected,
  resolve: unexpected,
  connectSaved: unexpected,
  removeProfile: unexpected,
  listProfiles: async () => profileList([profile], profile.id),
  refresh: unexpected,
  setTorrentPaused: unexpected,
  removeTorrents: unexpected,
  addTorrents: unexpected,
  defaultSavePath: unexpected,
  parseTorrentMetadata: async () => [],
  fetchTorrentMetadata: async () => ({ status: 'pending' }),
  fetchTorrentProperties: unexpected,
  fetchTorrentFiles: unexpected,
  fetchTorrentTrackers: unexpected,
  performTorrentContentAction: unexpected,
  setTorrentFilePriorities: unexpected,
  setTorrentCategory: unexpected,
  addTorrentTags: unexpected,
  removeTorrentTags: unexpected,
  fetchCategories: unexpected,
  fetchTags: unexpected,
  disconnect: async () => {},
  ...overrides,
})

const deferred = <T>() => {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((resolver) => {
    resolve = resolver
  })
  return { promise, resolve }
}

describe('useTorrentLibrary connection lifecycle', () => {
  beforeEach(() => {
    clearNuxtState()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('combines status and category filters and clears each independently', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => ({ ...snapshot, torrents: [
        torrent,
        { ...torrent, id: 'seed', status: 'seeding' },
        { ...torrent, id: 'movie', category: 'Movies' },
      ] }),
    }))
    await library.connect(connectionInput)
    library.chooseFilter('downloading')
    library.chooseCategory('Linux')
    expect(library.activeFilter.value).toBe('downloading')
    expect(library.visibleTorrents.value.map(torrent => torrent.id)).toEqual(['torrent-1'])
    library.chooseFilter('seeding')
    expect(library.activeCategory.value).toBe('Linux')
    expect(library.visibleTorrents.value.map(torrent => torrent.id)).toEqual(['seed'])
    library.chooseFilter('all')
    expect(library.visibleTorrents.value).toHaveLength(2)
    library.chooseCategory('')
    expect(library.visibleTorrents.value).toHaveLength(3)
  })

  it('ignores a refresh result superseded by going offline', async () => {
    const pendingRefresh = deferred<ConnectionSnapshot>()
    const adapter = createAdapter({
      connect: async () => snapshot,
      refresh: () => pendingRefresh.promise,
    })
    const library = useTorrentLibrary(adapter)

    expect(await library.connect(connectionInput)).toBe(true)
    const refreshResult = library.refresh()
    expect(library.refreshing.value).toBe(true)

    expect(await library.disconnect()).toBe(true)
    pendingRefresh.resolve(snapshot)

    expect(await refreshResult).toBe(false)
    expect(library.connectionStatus.value).toBe('disconnected')
    expect(library.torrents.value).toEqual([])
    expect(library.savedProfile.value).toEqual(profile)
    expect(library.refreshing.value).toBe(false)
  })

  it('keeps the saved profiles available for resolution after going offline', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      resolve: async () => ({ profiles: [profile], activeProfileId: profile.id, snapshot, error: null }),
    }))

    await library.connect(connectionInput)
    expect(await library.disconnect()).toBe(true)
    expect(library.savedProfiles.value).toEqual([profile])

    expect(await library.resolveConnection()).toBe(true)
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.torrents.value).toEqual([torrent])
  })

  it('retains the last snapshot as stale after a refresh failure', async () => {
    const adapter = createAdapter({
      connect: async () => snapshot,
      refresh: async () => {
        throw new Error('qBittorrent is unavailable')
      },
    })
    const library = useTorrentLibrary(adapter)

    await library.connect(connectionInput)

    expect(await library.refresh()).toBe(false)
    expect(library.connectionStatus.value).toBe('disconnected')
    expect(library.connectionError.value).toBe('qBittorrent is unavailable')
    expect(library.stale.value).toBe(true)
    expect(library.torrents.value).toEqual([torrent])
  })

  it('updates selected torrent activity from the authoritative response', async () => {
    const pausedTorrent = { ...torrent, status: 'paused' as const }
    const pausedSnapshot = { ...snapshot, torrents: [pausedTorrent] }
    const setTorrentPaused = vi.fn().mockResolvedValue(pausedSnapshot)
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      setTorrentPaused,
    }))

    await library.connect(connectionInput)

    expect(await library.setTorrentsPaused(['torrent-1', 'torrent-1'], true)).toBe(true)
    expect(setTorrentPaused).toHaveBeenCalledWith(['torrent-1'], true)
    expect(library.torrents.value).toEqual([pausedTorrent])
    expect(library.activityUpdating.value).toBe(false)
  })

  it('keeps the current snapshot connected when a torrent action fails', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      setTorrentPaused: async () => {
        throw new Error('qBittorrent rejected the request')
      },
    }))

    await library.connect(connectionInput)

    expect(await library.setTorrentsPaused(['torrent-1'], true)).toBe(false)
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.stale.value).toBe(false)
    expect(library.torrents.value).toEqual([torrent])
    expect(library.torrentActionError.value).toBe('qBittorrent rejected the request')
  })

  it('runs content actions without refreshing the torrent library', async () => {
    const performTorrentContentAction = vi.fn().mockResolvedValue(undefined)
    const refresh = vi.fn()
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      performTorrentContentAction,
      refresh,
    }))

    await library.connect(connectionInput)

    expect(await library.performTorrentContentAction('torrent-1', 'open', 3)).toBe(true)
    expect(performTorrentContentAction).toHaveBeenCalledWith('torrent-1', 3, 'open')
    expect(refresh).not.toHaveBeenCalled()
  })

  it('removes selected torrents and adopts the authoritative response', async () => {
    const emptiedSnapshot = { ...snapshot, torrents: [] }
    const removeTorrents = vi.fn().mockResolvedValue(emptiedSnapshot)
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      removeTorrents,
    }))

    await library.connect(connectionInput)

    expect(await library.removeTorrents(['torrent-1', ' torrent-1 '], true)).toBe(true)
    expect(removeTorrents).toHaveBeenCalledWith(['torrent-1'], true)
    expect(library.torrents.value).toEqual([])
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.activityUpdating.value).toBe(false)
  })

  it('keeps removed torrents visible when removal fails', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      removeTorrents: async () => {
        throw new Error('qBittorrent rejected the request')
      },
    }))

    await library.connect(connectionInput)

    expect(await library.removeTorrents(['torrent-1'], false)).toBe(false)
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.torrents.value).toEqual([torrent])
    expect(library.torrentActionError.value).toBe('qBittorrent rejected the request')
  })

  it('adds torrents and adopts the refreshed library', async () => {
    const outcome: AddTorrentsOutcome = { successCount: 1, failureCount: 0, pendingCount: 0, addedTorrentIds: ['new-1'] }
    const addedTorrent: Torrent = { ...torrent, id: 'new-1', name: 'New ISO' }
    const addTorrents = vi.fn().mockResolvedValue(outcome)
    const refresh = vi.fn().mockResolvedValue({ ...snapshot, torrents: [addedTorrent] })
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      addTorrents,
      refresh,
    }))

    await library.connect(connectionInput)

    const input: AddTorrentsInput = { urls: ['magnet:?xt=urn:btih:abc'], files: [], contentLayout: 'original' }
    expect(await library.addTorrents(input)).toEqual(outcome)
    expect(addTorrents).toHaveBeenCalledWith(input)
    expect(refresh).toHaveBeenCalledTimes(1)
    expect(library.torrents.value).toEqual([addedTorrent])
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.activityUpdating.value).toBe(false)
  })

  it('keeps the library intact when adding fails', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      addTorrents: async () => {
        throw new Error('qBittorrent rejected the request')
      },
    }))

    await library.connect(connectionInput)

    const input: AddTorrentsInput = { urls: ['magnet:?xt=urn:btih:abc'], files: [], contentLayout: 'original' }
    expect(await library.addTorrents(input)).toBeNull()
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.torrents.value).toEqual([torrent])
    expect(library.torrentActionError.value).toBe('qBittorrent rejected the request')
    expect(library.activityUpdating.value).toBe(false)
  })

  it('refuses to add without sources or a live connection', async () => {
    const addTorrents = vi.fn(async (): Promise<AddTorrentsOutcome> => ({
      successCount: 0,
      failureCount: 0,
      pendingCount: 0,
      addedTorrentIds: [],
    }))
    const library = useTorrentLibrary(createAdapter({ addTorrents }))

    expect(await library.addTorrents({ urls: [' magnet:?xt=urn:btih:abc '], files: [], contentLayout: 'original' })).toBeNull()
    expect(await library.addTorrents({ urls: [' '], files: [], contentLayout: 'original' })).toBeNull()
    expect(addTorrents).not.toHaveBeenCalled()
  })

  it.each(['setTorrentCategory', 'addTorrentTags', 'removeTorrentTags'] as const)('refreshes after %s and normalizes the selected ids', async (method) => {
    const mutate = vi.fn().mockResolvedValue(undefined)
    const refresh = vi.fn().mockResolvedValue({ ...snapshot, torrents: [] })
    const library = useTorrentLibrary(createAdapter({ connect: async () => snapshot, [method]: mutate, refresh }))
    await library.connect(connectionInput)

    const ids = [' torrent-1 ', 'torrent-1', ' ']
    const result = method === 'setTorrentCategory'
      ? await library[method](ids, 'Linux')
      : await library[method](ids, ['iso'])

    expect(result).toBe(true)
    expect(mutate).toHaveBeenCalledWith(['torrent-1'], method === 'setTorrentCategory' ? 'Linux' : ['iso'])
    expect(refresh).toHaveBeenCalledOnce()
    expect(library.torrents.value).toEqual([])
    expect(library.activityUpdating.value).toBe(false)
  })

  it('holds the mutation pending until file priorities and the subsequent refresh finish', async () => {
    const pendingRefresh = deferred<ConnectionSnapshot>()
    const setTorrentFilePriorities = vi.fn().mockResolvedValue(undefined)
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      setTorrentFilePriorities,
      refresh: () => pendingRefresh.promise,
    }))
    await library.connect(connectionInput)
    const priorities = [{ id: 0, priority: 0 as const }]
    const result = library.setTorrentFilePriorities('torrent-1', priorities)
    await Promise.resolve()
    expect(setTorrentFilePriorities).toHaveBeenCalledWith('torrent-1', priorities)
    expect(library.activityUpdating.value).toBe(true)
    pendingRefresh.resolve(snapshot)
    expect(await result).toBe(true)
    expect(library.activityUpdating.value).toBe(false)
  })

  it('does not report mutation success if its refresh is superseded by disconnecting', async () => {
    const pendingRefresh = deferred<ConnectionSnapshot>()
    const refresh = vi.fn().mockReturnValue(pendingRefresh.promise)
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      setTorrentCategory: async () => {},
      refresh,
    }))
    await library.connect(connectionInput)
    const result = library.setTorrentCategory(['torrent-1'], 'Linux')
    await Promise.resolve()
    expect(refresh).toHaveBeenCalledOnce()
    await library.disconnect()
    pendingRefresh.resolve(snapshot)
    expect(await result).toBe(false)
    expect(library.connectionStatus.value).toBe('disconnected')
    expect(library.torrents.value).toEqual([])
    expect(library.activityUpdating.value).toBe(false)
  })

  it('loads the default save path once per connection', async () => {
    const defaultSavePath = vi.fn().mockResolvedValue('C:/Downloads')
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      defaultSavePath,
    }))

    await library.connect(connectionInput)
    await library.loadDefaultSavePath()
    await library.loadDefaultSavePath()

    expect(defaultSavePath).toHaveBeenCalledTimes(1)
    expect(library.defaultSavePath.value).toBe('C:/Downloads')
  })

  it.each(['connect', 'connectProfile', 'resolveConnection'] as const)('reloads the default save path after %s changes the connection', async (method) => {
    const defaultSavePath = vi.fn().mockResolvedValueOnce('C:/Downloads').mockResolvedValue('/mnt/downloads')
    const library = useTorrentLibrary(createAdapter({
      connect: vi.fn().mockResolvedValueOnce(snapshot).mockResolvedValue(remoteSnapshot),
      connectSaved: async () => remoteSnapshot,
      resolve: async () => ({ profiles: [remoteProfile], activeProfileId: remoteProfile.id, snapshot: remoteSnapshot, error: null }),
      defaultSavePath,
    }))
    await library.connect(connectionInput)
    await library.loadDefaultSavePath()

    if (method === 'connect') await library.connect({ ...connectionInput, endpoint: remoteProfile.endpoint })
    else if (method === 'connectProfile') await library.connectProfile(remoteProfile.id)
    else await library.resolveConnection()

    expect(library.defaultSavePath.value).toBe('')
    await library.loadDefaultSavePath()
    expect(library.defaultSavePath.value).toBe('/mnt/downloads')
    expect(defaultSavePath).toHaveBeenCalledTimes(2)
  })

  it.each(['switch', 'disconnect', 'forget'] as const)('ignores an old save-path response after %s', async (action) => {
    const oldPath = deferred<string>()
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      connectSaved: async () => remoteSnapshot,
      removeProfile: async () => profileList([], null),
      defaultSavePath: vi.fn().mockReturnValueOnce(oldPath.promise).mockResolvedValue('/mnt/downloads'),
    }))
    await library.connect(connectionInput)
    const loading = library.loadDefaultSavePath()

    if (action === 'switch') {
      await library.connectProfile(remoteProfile.id)
      await library.loadDefaultSavePath()
    }
    else if (action === 'disconnect') await library.disconnect()
    else await library.forgetProfile(profile.id)

    oldPath.resolve('C:/Downloads')
    await loading
    expect(library.defaultSavePath.value).toBe(action === 'switch' ? '/mnt/downloads' : '')
  })

  it('keeps a pending save-path response valid across ordinary refreshes', async () => {
    const path = deferred<string>()
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      refresh: async () => snapshot,
      defaultSavePath: () => path.promise,
    }))
    await library.connect(connectionInput)
    const loading = library.loadDefaultSavePath()
    await library.refresh()
    path.resolve('C:/Downloads')
    await loading
    expect(library.defaultSavePath.value).toBe('C:/Downloads')
  })

  it('retries saved profiles with backoff and returns to normal polling after recovery', async () => {
    vi.useFakeTimers()
    const failedOutcome: ResolveOutcome = { profiles: [profile], activeProfileId: profile.id, snapshot: null, error: 'Not reachable' }
    const resolve = vi.fn()
      .mockResolvedValueOnce(failedOutcome)
      .mockResolvedValueOnce({ ...failedOutcome, error: 'Still unavailable' })
      .mockResolvedValueOnce({ profiles: [profile], activeProfileId: profile.id, snapshot, error: null })
    const refresh = vi.fn().mockResolvedValue(snapshot)
    const library = useTorrentLibrary(createAdapter({ resolve, refresh }))

    expect(await library.resolveConnection()).toBe(false)
    const stop = library.startAutoRefresh()

    await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)
    expect(resolve).toHaveBeenCalledTimes(2)
    expect(library.connectionStatus.value).toBe('disconnected')

    await vi.advanceTimersByTimeAsync(CONNECTION_RETRY_DELAYS_MS[0] - 1)
    expect(resolve).toHaveBeenCalledTimes(2)
    await vi.advanceTimersByTimeAsync(1)
    expect(resolve).toHaveBeenCalledTimes(3)
    expect(library.connectionStatus.value).toBe('connected')

    await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)
    expect(refresh).toHaveBeenCalledTimes(1)

    stop()
  })

  it('leaves a manual connection in progress when the polling timer fires', async () => {
    vi.useFakeTimers()
    const pendingConnection = deferred<ConnectionSnapshot>()
    const resolve = vi.fn()
    const refresh = vi.fn().mockResolvedValue(remoteSnapshot)
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      connectSaved: () => pendingConnection.promise,
      resolve,
      refresh,
    }))

    await library.connect(connectionInput)
    const stop = library.startAutoRefresh()
    try {
      const connection = library.connectProfile(remoteProfile.id)
      await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS * 2)
      expect(resolve).not.toHaveBeenCalled()
      expect(refresh).not.toHaveBeenCalled()
      expect(library.connectionStatus.value).toBe('connecting')

      pendingConnection.resolve(remoteSnapshot)
      expect(await connection).toBe(true)
      expect(library.connectionEndpoint.value).toBe(remoteProfile.endpoint)

      await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)
      expect(refresh).toHaveBeenCalledOnce()
    }
    finally {
      stop()
    }
  })

  it('caps repeated reconnect attempts at the longest retry delay', async () => {
    vi.useFakeTimers()
    const resolve = vi.fn().mockResolvedValue({ profiles: [profile], activeProfileId: profile.id, snapshot: null, error: 'Not reachable' })
    const library = useTorrentLibrary(createAdapter({ resolve }))

    await library.resolveConnection()
    const stop = library.startAutoRefresh()
    await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)

    for (const [index, delay] of CONNECTION_RETRY_DELAYS_MS.entries()) {
      await vi.advanceTimersByTimeAsync(delay)
      expect(resolve).toHaveBeenCalledTimes(index + 3)
    }

    const maximumDelay = CONNECTION_RETRY_DELAYS_MS.at(-1)!
    await vi.advanceTimersByTimeAsync(maximumDelay - 1)
    expect(resolve).toHaveBeenCalledTimes(CONNECTION_RETRY_DELAYS_MS.length + 2)
    await vi.advanceTimersByTimeAsync(1)
    expect(resolve).toHaveBeenCalledTimes(CONNECTION_RETRY_DELAYS_MS.length + 3)

    stop()
  })

  it('resolves the retained profiles and adopts the reachable one', async () => {
    const resolve = vi.fn().mockResolvedValue({
      profiles: [profile, remoteProfile],
      activeProfileId: remoteProfile.id,
      snapshot: remoteSnapshot,
      error: null,
    })
    const library = useTorrentLibrary(createAdapter({ resolve }))

    expect(await library.resolveConnection()).toBe(true)
    expect(library.savedProfiles.value).toEqual([profile, remoteProfile])
    expect(library.savedProfile.value).toEqual(remoteProfile)
    expect(library.connectionEndpoint.value).toBe(remoteProfile.endpoint)
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.torrents.value).toEqual([torrent])
  })

  it('keeps the retained profiles visible when nothing is reachable', async () => {
    const resolve = vi.fn().mockResolvedValue({
      profiles: [profile, remoteProfile],
      activeProfileId: profile.id,
      snapshot: null,
      error: 'qBittorrent is unavailable',
    })
    const library = useTorrentLibrary(createAdapter({
      resolve,
      listProfiles: async () => profileList([profile, remoteProfile], profile.id),
    }))

    expect(await library.resolveConnection()).toBe(false)
    expect(library.savedProfiles.value).toEqual([profile, remoteProfile])
    expect(library.savedProfile.value).toEqual(profile)
    expect(library.connectionStatus.value).toBe('disconnected')
    expect(library.connectionError.value).toBe('qBittorrent is unavailable')
  })

  it('switches the active connection to a retained profile', async () => {
    const library = useTorrentLibrary(createAdapter({
      connectSaved: async () => remoteSnapshot,
      listProfiles: async () => profileList([profile, remoteProfile], remoteProfile.id),
    }))

    expect(await library.connectProfile(remoteProfile.id)).toBe(true)
    expect(library.savedProfile.value).toEqual(remoteProfile)
    expect(library.connectionEndpoint.value).toBe(remoteProfile.endpoint)
    expect(library.connectionStatus.value).toBe('connected')
  })

  it('forgets one profile without touching the retained others', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      listProfiles: async () => profileList([profile, remoteProfile], profile.id),
      removeProfile: async () => profileList([remoteProfile], null),
    }))

    await library.connect(connectionInput)
    expect(await library.forgetProfile(profile.id)).toBe(true)

    expect(library.savedProfiles.value).toEqual([remoteProfile])
    expect(library.savedProfile.value).toBeNull()
    expect(library.connectionStatus.value).toBe('disconnected')
    expect(library.torrents.value).toEqual([])
  })

  it('keeps the connection when forgetting a non-active profile', async () => {
    const library = useTorrentLibrary(createAdapter({
      connect: async () => snapshot,
      listProfiles: async () => profileList([profile], profile.id),
      removeProfile: async () => profileList([profile], profile.id),
    }))

    await library.connect(connectionInput)
    expect(await library.forgetProfile(remoteProfile.id)).toBe(true)

    expect(library.savedProfiles.value).toEqual([profile])
    expect(library.savedProfile.value).toEqual(profile)
    expect(library.connectionStatus.value).toBe('connected')
    expect(library.torrents.value).toEqual([torrent])
  })
})

describe('useTorrentLibrary placeholder library', () => {
  beforeEach(() => {
    clearNuxtState()
    localStorage.clear()
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  })

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
  })

  it('shows the placeholder library while the setting is enabled', () => {
    localStorage.setItem('cloudburst:placeholder-enabled', 'true')

    const library = useTorrentLibrary()

    expect(library.torrents.value.length).toBeGreaterThan(0)
    expect(library.connectionStatus.value).toBe('disconnected')
  })

  it('keeps the real library visible when the setting is disabled', () => {
    const library = useTorrentLibrary()

    expect(library.torrents.value).toEqual([])
  })

  it('does not show placeholders for custom adapters', () => {
    localStorage.setItem('cloudburst:placeholder-enabled', 'true')

    const library = useTorrentLibrary(createAdapter())

    expect(library.torrents.value).toEqual([])
  })

  it('swaps to the placeholder list at runtime without disturbing the real library', async () => {
    useState<Torrent[]>('torrent-library', () => []).value = [torrent]
    const library = useTorrentLibrary()
    expect(library.torrents.value).toEqual([torrent])

    usePlaceholderSetting().setPlaceholderEnabled(true)
    await nextTick()
    expect(library.torrents.value.length).toBeGreaterThan(0)

    usePlaceholderSetting().setPlaceholderEnabled(false)
    await nextTick()

    expect(library.torrents.value).toEqual([torrent])
  })

  it('hides placeholder torrents when the setting is disabled at runtime', async () => {
    localStorage.setItem('cloudburst:placeholder-enabled', 'true')
    const library = useTorrentLibrary()
    expect(library.torrents.value.length).toBeGreaterThan(0)

    usePlaceholderSetting().setPlaceholderEnabled(false)
    await nextTick()

    expect(library.torrents.value).toEqual([])
  })
})
