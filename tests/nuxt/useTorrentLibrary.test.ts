import { clearNuxtState } from '#app'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { QbittorrentAdapter } from '~/adapters/qbittorrent'
import { CONNECTION_POLL_INTERVAL_MS, CONNECTION_RETRY_DELAYS_MS, useTorrentLibrary } from '~/composables/useTorrentLibrary'
import type { AddTorrentsInput, AddTorrentsOutcome, ConnectionInput, ConnectionProfile, ConnectionSnapshot, Torrent } from '~/types/torrent'

const connectionInput: ConnectionInput = {
  endpoint: 'http://localhost:8080',
  authenticationMode: 'apiKey',
  apiKey: 'qbt_0000000000000000000000000000',
}

const profile: ConnectionProfile = {
  endpoint: 'http://localhost:8080',
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

const unexpected = async (): Promise<never> => {
  throw new Error('Unexpected adapter call')
}

const createAdapter = (overrides: Partial<QbittorrentAdapter> = {}): QbittorrentAdapter => ({
  connect: unexpected,
  restore: unexpected,
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

  it('ignores a refresh result superseded by forgetting the connection', async () => {
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
    expect(library.savedProfile.value).toBeNull()
    expect(library.refreshing.value).toBe(false)
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

  it('retries a saved profile with backoff and returns to normal polling after recovery', async () => {
    vi.useFakeTimers()
    const restore = vi.fn()
      .mockResolvedValueOnce({ profile, snapshot: null, error: 'Not reachable' })
      .mockResolvedValueOnce({ profile, snapshot: null, error: 'Still unavailable' })
      .mockResolvedValueOnce({ profile, snapshot, error: null })
    const refresh = vi.fn().mockResolvedValue(snapshot)
    const library = useTorrentLibrary(createAdapter({ restore, refresh }))

    expect(await library.restoreSavedConnection()).toBe(false)
    const stop = library.startAutoRefresh()

    await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)
    expect(restore).toHaveBeenCalledTimes(2)
    expect(library.connectionStatus.value).toBe('disconnected')

    await vi.advanceTimersByTimeAsync(CONNECTION_RETRY_DELAYS_MS[0] - 1)
    expect(restore).toHaveBeenCalledTimes(2)
    await vi.advanceTimersByTimeAsync(1)
    expect(restore).toHaveBeenCalledTimes(3)
    expect(library.connectionStatus.value).toBe('connected')

    await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)
    expect(refresh).toHaveBeenCalledTimes(1)

    stop()
  })

  it('caps repeated reconnect attempts at the longest retry delay', async () => {
    vi.useFakeTimers()
    const restore = vi.fn().mockResolvedValue({ profile, snapshot: null, error: 'Not reachable' })
    const library = useTorrentLibrary(createAdapter({ restore }))

    await library.restoreSavedConnection()
    const stop = library.startAutoRefresh()
    await vi.advanceTimersByTimeAsync(CONNECTION_POLL_INTERVAL_MS)

    for (const [index, delay] of CONNECTION_RETRY_DELAYS_MS.entries()) {
      await vi.advanceTimersByTimeAsync(delay)
      expect(restore).toHaveBeenCalledTimes(index + 3)
    }

    const maximumDelay = CONNECTION_RETRY_DELAYS_MS.at(-1)!
    await vi.advanceTimersByTimeAsync(maximumDelay - 1)
    expect(restore).toHaveBeenCalledTimes(CONNECTION_RETRY_DELAYS_MS.length + 2)
    await vi.advanceTimersByTimeAsync(1)
    expect(restore).toHaveBeenCalledTimes(CONNECTION_RETRY_DELAYS_MS.length + 3)

    stop()
  })
})
