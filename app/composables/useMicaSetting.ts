const MICA_STORAGE_KEY = 'cloudburst:mica-enabled'

export const useMicaSetting = () => {
  const micaEnabled = useState('mica-setting', () => true)

  const isWindowsDesktop = typeof window !== 'undefined'
    && '__TAURI_INTERNALS__' in window
    && navigator.userAgent.includes('Windows')

  const applyMicaClass = () => {
    if (typeof document === 'undefined') return
    document.documentElement.classList.toggle('mica', isWindowsDesktop && micaEnabled.value)
  }

  const loadMicaSetting = () => {
    const stored = localStorage.getItem(MICA_STORAGE_KEY)
    if (stored !== null) micaEnabled.value = stored === 'true'
    applyMicaClass()
  }

  const setMicaEnabled = (enabled: boolean) => {
    micaEnabled.value = enabled
    localStorage.setItem(MICA_STORAGE_KEY, String(enabled))
    applyMicaClass()
  }

  return {
    micaEnabled,
    canUseMica: isWindowsDesktop,
    loadMicaSetting,
    setMicaEnabled,
  }
}
