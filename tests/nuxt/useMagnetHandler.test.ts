import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { useMagnetHandler } from '~/composables/useMagnetHandler'

const magnetMocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: magnetMocks.invoke,
}))

const stubWindowsDesktop = () => {
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  Object.defineProperty(window.navigator, 'userAgent', {
    value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) WebView2',
    configurable: true,
  })
}

describe('useMagnetHandler', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.clearAllMocks()
    stubWindowsDesktop()
  })

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
    Reflect.deleteProperty(window.navigator, 'userAgent')
  })

  it('offers setup when another app handles magnets and remembers dismissal', async () => {
    magnetMocks.invoke.mockResolvedValue('otherProgram')
    const handler = useMagnetHandler()

    await handler.checkHandler()
    expect(handler.hintOpen.value).toBe(true)
    handler.hintOpen.value = false
    await nextTick()
    expect(localStorage.getItem('cloudburst:magnet-hint-dismissed')).toBe('true')

    await handler.checkHandler()
    expect(magnetMocks.invoke).toHaveBeenCalledTimes(1)
    handler.showSetup()
    expect(handler.hintOpen.value).toBe(true)
  })

  it('keeps setup quiet when Cloudburst is already the default', async () => {
    magnetMocks.invoke.mockResolvedValue('cloudburstDefault')
    const handler = useMagnetHandler()

    await handler.checkHandler()
    expect(handler.hintOpen.value).toBe(false)
  })

  it('opens Windows default apps settings', async () => {
    magnetMocks.invoke.mockResolvedValue(undefined)
    const handler = useMagnetHandler()
    handler.showSetup()

    await handler.openSettings()

    expect(magnetMocks.invoke).toHaveBeenCalledWith('open_default_apps_settings')
    expect(handler.hintOpen.value).toBe(false)
  })
})
