import { invoke } from '@tauri-apps/api/core'
import type { MagnetHandlerStatus } from '~/types/torrent'

const DISMISSED_KEY = 'cloudburst:magnet-hint-dismissed'

export const useMagnetHandler = () => {
  const hintOpen = ref(false)
  const supported = typeof window !== 'undefined'
    && '__TAURI_INTERNALS__' in window
    && navigator.userAgent.includes('Windows')

  const checkHandler = async () => {
    if (!supported || localStorage.getItem(DISMISSED_KEY) === 'true') return
    try {
      hintOpen.value = await invoke<MagnetHandlerStatus>('magnet_handler_status') !== 'cloudburstDefault'
    }
    catch {
      // Handler detection is best-effort; adding torrents works regardless.
    }
  }

  const showSetup = () => {
    if (supported) hintOpen.value = true
  }

  watch(hintOpen, (open, wasOpen) => {
    if (!open && wasOpen) localStorage.setItem(DISMISSED_KEY, 'true')
  })

  const openSettings = async () => {
    await invoke('open_default_apps_settings')
    hintOpen.value = false
  }

  return { hintOpen, supported, checkHandler, showSetup, openSettings }
}
