import { invoke } from '@tauri-apps/api/core'
import type { Torrent } from '~/types/torrent'

const TORRENT_NOTIFICATIONS_STORAGE_KEY = 'cloudburst:torrent-notifications-enabled'

interface TorrentNotification {
  title: string
  body: string
}

export const collectTorrentNotifications = (
  previous: readonly Torrent[],
  current: readonly Torrent[],
): TorrentNotification[] => {
  const previousById = new Map(previous.map(torrent => [torrent.id, torrent]))
  const notifications: TorrentNotification[] = []

  for (const torrent of current) {
    const earlier = previousById.get(torrent.id)
    if (!earlier) continue

    if (earlier.status !== 'error' && torrent.status === 'error') {
      notifications.push({
        title: 'Torrent needs attention',
        body: `${torrent.name} encountered an error.`,
      })
    }
    else if (earlier.progress < 100 && torrent.progress >= 100) {
      notifications.push({
        title: 'Torrent completed',
        body: `${torrent.name} finished downloading.`,
      })
    }
  }

  return notifications
}

export const useTorrentNotificationSetting = () => {
  const notificationsEnabled = useState<boolean>('torrent-notifications-setting', () =>
    localStorage.getItem(TORRENT_NOTIFICATIONS_STORAGE_KEY) === 'true')
  const canUseNotifications = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

  const setNotificationsEnabled = (enabled: boolean) => {
    notificationsEnabled.value = enabled
    localStorage.setItem(TORRENT_NOTIFICATIONS_STORAGE_KEY, String(enabled))
  }

  const sendTorrentNotification = async (notification: TorrentNotification) => {
    if (!notificationsEnabled.value || !canUseNotifications) return

    try {
      await invoke('send_torrent_notification', { ...notification })
    }
    catch {
      // A notification failure should not interrupt torrent refreshes.
    }
  }

  return {
    notificationsEnabled,
    canUseNotifications,
    setNotificationsEnabled,
    sendTorrentNotification,
  }
}
