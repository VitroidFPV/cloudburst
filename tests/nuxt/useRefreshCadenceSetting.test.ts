import { clearNuxtState } from '#app'
import { beforeEach, describe, expect, it } from 'vitest'
import { REFRESH_CADENCE_INTERVALS_MS, useRefreshCadenceSetting } from '~/composables/useRefreshCadenceSetting'

const REFRESH_CADENCE_STORAGE_KEY = 'cloudburst:refresh-cadence'

describe('useRefreshCadenceSetting', () => {
  beforeEach(() => {
    clearNuxtState()
    localStorage.clear()
  })

  it('defaults to the normal cadence', () => {
    expect(useRefreshCadenceSetting().refreshCadence.value).toBe('normal')
  })

  it('persists and restores a chosen cadence', () => {
    const setting = useRefreshCadenceSetting()

    setting.setRefreshCadence('fast')

    expect(setting.refreshCadence.value).toBe('fast')
    expect(localStorage.getItem(REFRESH_CADENCE_STORAGE_KEY)).toBe('fast')

    clearNuxtState()
    expect(useRefreshCadenceSetting().refreshCadence.value).toBe('fast')
  })

  it('falls back to the normal cadence for unknown stored values', () => {
    localStorage.setItem(REFRESH_CADENCE_STORAGE_KEY, 'instant')

    expect(useRefreshCadenceSetting().refreshCadence.value).toBe('normal')
  })

  it('maps every cadence to its poll interval', () => {
    expect(REFRESH_CADENCE_INTERVALS_MS.fast).toBe(2_000)
    expect(REFRESH_CADENCE_INTERVALS_MS.normal).toBe(5_000)
    expect(REFRESH_CADENCE_INTERVALS_MS.slow).toBe(15_000)
  })
})
