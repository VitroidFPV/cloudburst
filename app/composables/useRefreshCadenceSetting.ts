export const REFRESH_CADENCES = ['fast', 'normal', 'slow'] as const

export type RefreshCadence = typeof REFRESH_CADENCES[number]

export const REFRESH_CADENCE_INTERVALS_MS: Record<RefreshCadence, number> = {
  fast: 2_000,
  normal: 5_000,
  slow: 15_000,
}

const REFRESH_CADENCE_STORAGE_KEY = 'cloudburst:refresh-cadence'

const isRefreshCadence = (value: string | null): value is RefreshCadence =>
  REFRESH_CADENCES.some(cadence => cadence === value)

export const useRefreshCadenceSetting = () => {
  const refreshCadence = useState<RefreshCadence>('refresh-cadence-setting', () => {
    const stored = localStorage.getItem(REFRESH_CADENCE_STORAGE_KEY)
    return isRefreshCadence(stored) ? stored : 'normal'
  })

  const setRefreshCadence = (cadence: RefreshCadence) => {
    refreshCadence.value = cadence
    localStorage.setItem(REFRESH_CADENCE_STORAGE_KEY, cadence)
  }

  return {
    refreshCadence,
    setRefreshCadence,
  }
}
