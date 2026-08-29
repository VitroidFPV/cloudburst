import { clearNuxtState } from '#app'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { useMicaSetting } from '~/composables/useMicaSetting'

const MICA_STORAGE_KEY = 'cloudburst:mica-enabled'

const stubWindowsDesktop = () => {
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  Object.defineProperty(window.navigator, 'userAgent', {
    value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) WebView2',
    configurable: true,
  })
}

const clearWindowsDesktopStub = () => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
  Reflect.deleteProperty(window.navigator, 'userAgent')
}

describe('useMicaSetting', () => {
  beforeEach(() => {
    clearNuxtState()
    localStorage.clear()
    document.documentElement.classList.remove('mica')
  })

  afterEach(() => {
    clearWindowsDesktopStub()
    document.documentElement.classList.remove('mica')
  })

  it('defaults to enabled outside the Windows desktop and never applies the class', () => {
    const { micaEnabled, canUseMica } = useMicaSetting()

    expect(micaEnabled.value).toBe(true)
    expect(canUseMica).toBe(false)

    useMicaSetting().loadMicaSetting()
    expect(document.documentElement.classList.contains('mica')).toBe(false)
  })

  it('applies the class only while enabled inside the Windows desktop', () => {
    stubWindowsDesktop()
    const setting = useMicaSetting()

    setting.loadMicaSetting()
    expect(document.documentElement.classList.contains('mica')).toBe(true)

    setting.setMicaEnabled(false)
    expect(document.documentElement.classList.contains('mica')).toBe(false)
    expect(localStorage.getItem(MICA_STORAGE_KEY)).toBe('false')

    setting.setMicaEnabled(true)
    expect(document.documentElement.classList.contains('mica')).toBe(true)
    expect(localStorage.getItem(MICA_STORAGE_KEY)).toBe('true')
  })

  it('restores the persisted choice when the desktop shell loads', () => {
    stubWindowsDesktop()
    localStorage.setItem(MICA_STORAGE_KEY, 'false')

    useMicaSetting().loadMicaSetting()

    expect(document.documentElement.classList.contains('mica')).toBe(false)
    expect(useMicaSetting().micaEnabled.value).toBe(false)
  })
})
