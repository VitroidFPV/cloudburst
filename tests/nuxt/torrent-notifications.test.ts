import { clearNuxtState } from '#app'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Torrent } from '~/types/torrent'
import { collectTorrentNotifications } from '~/composables/useTorrentNotificationSetting'

const notificationMocks = vi.hoisted(() => ({
  invoke: vi.fn(async () => {}),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: notificationMocks.invoke,
}))

const torrent = (overrides: Partial<Torrent> = {}): Torrent => ({
  id: 'torrent-1',
  name: 'Cloudburst Linux',
  status: 'downloading',
  progress: 40,
  size: 1_000,
  downloaded: 400,
  downSpeed: 10,
  upSpeed: 0,
  etaSeconds: 60,
  ratio: 0,
  seeds: 1,
  peers: 1,
  category: '',
  tags: [],
  addedOn: 0,
  savePath: '/downloads',
  ...overrides,
})

describe('collectTorrentNotifications', () => {
  it('does not notify for the initial library or a newly added torrent', () => {
    expect(collectTorrentNotifications([], [torrent()])).toEqual([])
    expect(collectTorrentNotifications([torrent()], [torrent(), torrent({ id: 'torrent-2' })])).toEqual([])
  })

  it('notifies when a known torrent completes', () => {
    expect(collectTorrentNotifications(
      [torrent({ progress: 99 })],
      [torrent({ progress: 100, status: 'seeding' })],
    )).toEqual([{
      title: 'Torrent completed',
      body: 'Cloudburst Linux finished downloading.',
    }])
  })

  it('notifies once when a known torrent enters an error state', () => {
    const failed = torrent({ status: 'error' })

    expect(collectTorrentNotifications([torrent()], [failed])).toEqual([{
      title: 'Torrent needs attention',
      body: 'Cloudburst Linux encountered an error.',
    }])
    expect(collectTorrentNotifications([failed], [failed])).toEqual([])
  })

  it('prefers an error notification when completion and failure happen together', () => {
    expect(collectTorrentNotifications(
      [torrent({ progress: 99 })],
      [torrent({ progress: 100, status: 'error' })],
    )[0]?.title).toBe('Torrent needs attention')
  })
})

describe('useTorrentNotificationSetting', () => {
  beforeEach(() => {
    clearNuxtState()
    vi.clearAllMocks()
    localStorage.clear()
    localStorage.setItem('cloudburst:torrent-notifications-enabled', 'true')
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  })

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
  })

  it('routes notifications through Cloudbursts native sender', async () => {
    const notification = {
      title: 'Torrent completed',
      body: 'Cloudburst Linux finished downloading.',
    }

    await useTorrentNotificationSetting().sendTorrentNotification(notification)
    expect(notificationMocks.invoke).toHaveBeenCalledWith('send_torrent_notification', {
      title: notification.title,
      body: notification.body,
    })
  })
})
