import { isUiDebugActive } from '~/utils/ui-debug'

const PLACEHOLDER_STORAGE_KEY = 'cloudburst:placeholder-enabled'

const isPlaceholderFlag = (value: string | null): value is 'true' | 'false' =>
  value === 'true' || value === 'false'

export const usePlaceholderSetting = (forced: boolean = isUiDebugActive()) => {
  const placeholderEnabled = useState<boolean>('placeholder-setting', () => {
    const stored = localStorage.getItem(PLACEHOLDER_STORAGE_KEY)
    return isPlaceholderFlag(stored) ? stored === 'true' : false
  })

  const placeholderForced = forced

  const showPlaceholder = computed(() => placeholderForced || placeholderEnabled.value)

  const setPlaceholderEnabled = (enabled: boolean) => {
    placeholderEnabled.value = enabled
    localStorage.setItem(PLACEHOLDER_STORAGE_KEY, String(enabled))
  }

  return {
    placeholderEnabled,
    placeholderForced,
    showPlaceholder,
    setPlaceholderEnabled,
  }
}
