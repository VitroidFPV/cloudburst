import { clearNuxtState } from '#app'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppearanceSetting } from '~/composables/useAppearanceSetting'

const windowMocks = vi.hoisted(() => ({
  clearEffects: vi.fn(async () => {}),
  setBackgroundColor: vi.fn(async () => {}),
  setEffects: vi.fn(async () => {}),
}))

vi.mock('@tauri-apps/api/window', () => ({
  Effect: { Mica: 'mica' },
  getCurrentWindow: () => windowMocks,
}))

const APPEARANCE_STORAGE_KEY = 'cloudburst:appearance-mode'
const LEGACY_MICA_STORAGE_KEY = 'cloudburst:mica-enabled'
const APPEARANCE_CLASSES = ['appearance-off', 'appearance-toned', 'appearance-mica']

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

const activeAppearanceClasses = () => APPEARANCE_CLASSES.filter(className =>
  document.documentElement.classList.contains(className),
)

describe('useAppearanceSetting', () => {
  beforeEach(() => {
    clearNuxtState()
    vi.clearAllMocks()
    localStorage.clear()
    document.documentElement.classList.remove(...APPEARANCE_CLASSES)
  })

  afterEach(() => {
    clearWindowsDesktopStub()
    document.documentElement.classList.remove(...APPEARANCE_CLASSES)
  })

  it('defaults to Mica but keeps non-Windows environments opaque', () => {
    const { appearanceMode, canUseWindowMaterials, loadAppearanceSetting } = useAppearanceSetting()

    expect(appearanceMode.value).toBe('mica')
    expect(canUseWindowMaterials).toBe(false)

    loadAppearanceSetting()
    expect(activeAppearanceClasses()).toEqual(['appearance-off'])
  })

  it.each(['off', 'toned', 'mica'] as const)('applies and persists the %s mode on Windows', (mode) => {
    stubWindowsDesktop()
    const setting = useAppearanceSetting()

    setting.setAppearanceMode(mode)

    expect(setting.appearanceMode.value).toBe(mode)
    expect(activeAppearanceClasses()).toEqual([`appearance-${mode}`])
    expect(localStorage.getItem(APPEARANCE_STORAGE_KEY)).toBe(mode)
  })

  it('sets the native window background before clearing Mica in Off mode', async () => {
    stubWindowsDesktop()
    document.documentElement.classList.add('dark')

    useAppearanceSetting().setAppearanceMode('off')

    await vi.waitFor(() => expect(windowMocks.clearEffects).toHaveBeenCalledOnce())
    expect(windowMocks.setBackgroundColor).toHaveBeenCalledWith([16, 16, 18, 255])
    expect(windowMocks.setBackgroundColor.mock.invocationCallOrder[0])
      .toBeLessThan(windowMocks.clearEffects.mock.invocationCallOrder[0]!)
  })

  it('restores a persisted mode when the desktop shell loads', () => {
    stubWindowsDesktop()
    localStorage.setItem(APPEARANCE_STORAGE_KEY, 'toned')

    useAppearanceSetting().loadAppearanceSetting()

    expect(activeAppearanceClasses()).toEqual(['appearance-toned'])
    expect(useAppearanceSetting().appearanceMode.value).toBe('toned')
  })

  it.each([
    ['false', 'off'],
    ['true', 'mica'],
  ] as const)('migrates the legacy Mica value %s to %s', (legacyValue, expectedMode) => {
    stubWindowsDesktop()
    localStorage.setItem(LEGACY_MICA_STORAGE_KEY, legacyValue)

    useAppearanceSetting().loadAppearanceSetting()

    expect(useAppearanceSetting().appearanceMode.value).toBe(expectedMode)
    expect(localStorage.getItem(APPEARANCE_STORAGE_KEY)).toBe(expectedMode)
    expect(localStorage.getItem(LEGACY_MICA_STORAGE_KEY)).toBeNull()
    expect(activeAppearanceClasses()).toEqual([`appearance-${expectedMode}`])
  })
})
