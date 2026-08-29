export const APPEARANCE_MODES = ['off', 'toned', 'mica'] as const

export type AppearanceMode = typeof APPEARANCE_MODES[number]

const APPEARANCE_STORAGE_KEY = 'cloudburst:appearance-mode'
const LEGACY_MICA_STORAGE_KEY = 'cloudburst:mica-enabled'
const APPEARANCE_CLASSES = APPEARANCE_MODES.map(mode => `appearance-${mode}`)
const DEFAULT_WINDOW_BACKGROUNDS = {
  dark: [16, 16, 18, 255],
  light: [255, 255, 255, 255],
} as const

const isAppearanceMode = (value: string | null): value is AppearanceMode =>
  APPEARANCE_MODES.some(mode => mode === value)

const getWindowBackgroundColor = (): [number, number, number, number] => {
  const root = document.documentElement
  const cssColor = getComputedStyle(root).getPropertyValue('--cloudburst-window-bg').trim()
  const channels = cssColor.split(/\s+/).map(Number)

  if (channels.length === 3 && channels.every(channel => Number.isFinite(channel))) {
    return [channels[0]!, channels[1]!, channels[2]!, 255]
  }

  return [...DEFAULT_WINDOW_BACKGROUNDS[root.classList.contains('dark') ? 'dark' : 'light']]
}

export const useAppearanceSetting = () => {
  const appearanceMode = useState<AppearanceMode>('appearance-setting', () => 'mica')

  const canUseWindowMaterials = typeof window !== 'undefined'
    && '__TAURI_INTERNALS__' in window
    && navigator.userAgent.includes('Windows')

  const applyAppearanceClass = () => {
    if (typeof document === 'undefined') return

    const root = document.documentElement
    root.classList.remove(...APPEARANCE_CLASSES)
    root.classList.add(`appearance-${canUseWindowMaterials ? appearanceMode.value : 'off'}`)
  }

  const applyWindowEffect = async () => {
    if (!canUseWindowMaterials) return

    const requestedMode = appearanceMode.value

    try {
      const [{ Effect, getCurrentWindow }, { invoke }] = await Promise.all([
        import('@tauri-apps/api/window'),
        import('@tauri-apps/api/core'),
      ])
      if (requestedMode !== appearanceMode.value) return

      const appWindow = getCurrentWindow()
      if (requestedMode === 'off') {
        const backgroundColor = getWindowBackgroundColor()
        await appWindow.setBackgroundColor(backgroundColor)
        if (requestedMode !== appearanceMode.value) return
        await appWindow.clearEffects()
        if (requestedMode !== appearanceMode.value) return
        await invoke('set_window_caption_color', { color: backgroundColor.slice(0, 3) })
      }
      else {
        await invoke('set_window_caption_color', { color: null })
        if (requestedMode !== appearanceMode.value) return
        await appWindow.setEffects({ effects: [Effect.Mica] })
      }
    }
    catch {
      // CSS still provides a readable fallback if the native effect is unavailable.
    }
  }

  const loadAppearanceSetting = () => {
    const stored = localStorage.getItem(APPEARANCE_STORAGE_KEY)

    if (isAppearanceMode(stored)) {
      appearanceMode.value = stored
    }
    else {
      const legacyMicaEnabled = localStorage.getItem(LEGACY_MICA_STORAGE_KEY)
      if (legacyMicaEnabled !== null) {
        appearanceMode.value = legacyMicaEnabled === 'false' ? 'off' : 'mica'
        localStorage.setItem(APPEARANCE_STORAGE_KEY, appearanceMode.value)
        localStorage.removeItem(LEGACY_MICA_STORAGE_KEY)
      }
    }

    applyAppearanceClass()
    void applyWindowEffect()
  }

  const setAppearanceMode = (mode: AppearanceMode) => {
    appearanceMode.value = mode
    localStorage.setItem(APPEARANCE_STORAGE_KEY, mode)
    applyAppearanceClass()
    void applyWindowEffect()
  }

  return {
    appearanceMode,
    canUseWindowMaterials,
    loadAppearanceSetting,
    setAppearanceMode,
  }
}
