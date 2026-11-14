interface SystemColors {
  scheme: number | null
  accent: number[] | null
  colors: Record<string, number[]>
}

export const useSystemColors = () => {
  const { appearanceMode, canUseSystemColors } = useAppearanceSetting()
  const colorMode = useColorMode()
  let unlisten: (() => void) | undefined
  let disposed = false
  let revision = 0
  const root = () => document.documentElement
  const clear = () => {
    root().classList.remove('system-palette')
    for (const key of Array.from(root().style)) {
      if (key.startsWith('--system-')) root().style.removeProperty(key)
    }
  }
  const refresh = async () => {
    const current = ++revision
    if (!canUseSystemColors || appearanceMode.value !== 'system') { clear(); return }
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const palette = await invoke<SystemColors>('system_colors')
      if (disposed || current !== revision) return
      clear()
      const { colors, accent, scheme } = palette
      const rgb = (value: number[]) => `rgb(${value.join(' ')})`
      for (const [key, value] of Object.entries(colors)) {
        root().style.setProperty(`--system-${key}`, rgb(value))
      }
      if (accent) root().style.setProperty('--system-accent', rgb(accent))
      if (colors.window) {
        root().style.setProperty('--system-window-channels', colors.window.join(' '))
        root().classList.toggle('dark', colors.window.reduce((sum, c, i) => sum + c * [0.2126, 0.7152, 0.0722][i]!, 0) < 128)
      }
      else if (scheme === 1 || scheme === 2) {
        root().classList.toggle('dark', scheme === 1)
      }
      root().classList.add('system-palette')
    }
    catch {
      // Keep the normal app theme when the desktop portal is unavailable.
    }
  }
  watch(appearanceMode, () => {
    if (appearanceMode.value !== 'system') {
      root().classList.toggle('dark', colorMode.value === 'dark')
    }
    void refresh()
  })
  watch(() => colorMode.value, () => { void refresh() })
  onMounted(async () => {
    if (!canUseSystemColors) return
    try {
      const { listen } = await import('@tauri-apps/api/event')
      const stop = await listen('system-colors-changed', () => { void refresh() })
      if (disposed) { stop(); return }
      unlisten = stop
    }
    catch {
      // Initial palette loading can still work without live updates.
    }
    finally {
      if (!disposed) void refresh()
    }
  })
  onBeforeUnmount(() => { disposed = true; unlisten?.(); clear() })
}
