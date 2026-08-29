import { clearNuxtState } from '#app'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { usePlaceholderSetting } from '~/composables/usePlaceholderSetting'

const PLACEHOLDER_STORAGE_KEY = 'cloudburst:placeholder-enabled'

const stubWindowsDesktop = () => {
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
}

const clearWindowsDesktopStub = () => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
}

describe('usePlaceholderSetting', () => {
  beforeEach(() => {
    clearNuxtState()
    localStorage.clear()
  })

  afterEach(clearWindowsDesktopStub)

  it('defaults to disabled on the desktop shell', () => {
    stubWindowsDesktop()
    const setting = usePlaceholderSetting()

    expect(setting.placeholderEnabled.value).toBe(false)
    expect(setting.showPlaceholder.value).toBe(false)
  })

  it('persists and restores the enabled state on the desktop shell', () => {
    stubWindowsDesktop()

    const setting = usePlaceholderSetting()
    setting.setPlaceholderEnabled(true)

    expect(setting.showPlaceholder.value).toBe(true)
    expect(localStorage.getItem(PLACEHOLDER_STORAGE_KEY)).toBe('true')

    clearNuxtState()
    expect(usePlaceholderSetting().placeholderEnabled.value).toBe(true)
  })

  it('keeps a stored disabled state off', () => {
    stubWindowsDesktop()
    localStorage.setItem(PLACEHOLDER_STORAGE_KEY, 'false')

    const setting = usePlaceholderSetting()

    expect(setting.placeholderEnabled.value).toBe(false)
    expect(setting.showPlaceholder.value).toBe(false)
  })

  it('ignores stored values other than the enabled flags', () => {
    stubWindowsDesktop()
    localStorage.setItem(PLACEHOLDER_STORAGE_KEY, 'yes')

    expect(usePlaceholderSetting().placeholderEnabled.value).toBe(false)
  })

  it('is forced on in the browser preview regardless of the stored setting', () => {
    localStorage.setItem(PLACEHOLDER_STORAGE_KEY, 'false')

    const setting = usePlaceholderSetting(true)

    expect(setting.placeholderForced).toBe(true)
    expect(setting.showPlaceholder.value).toBe(true)
  })
})
