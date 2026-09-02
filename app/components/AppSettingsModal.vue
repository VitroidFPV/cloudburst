<script setup lang="ts">
import type { AppearanceMode } from '~/composables/useAppearanceSetting'
import type { RefreshCadence } from '~/composables/useRefreshCadenceSetting'

const { appearanceMode, canUseWindowMaterials, setAppearanceMode } = useAppearanceSetting()
const { placeholderEnabled, placeholderForced, setPlaceholderEnabled } = usePlaceholderSetting()
const { refreshCadence, setRefreshCadence } = useRefreshCadenceSetting()
const {
  notificationsEnabled,
  canUseNotifications,
  setNotificationsEnabled,
} = useTorrentNotificationSetting()
const colorMode = useColorMode()

const open = defineModel<boolean>('open', { default: false })

const appearanceOptions = computed(() => [
  {
    label: 'Flat',
    value: 'off' as const,
    icon: 'i-lucide-square',
  },
  {
    label: 'Toned',
    value: 'toned' as const,
    icon: 'i-lucide-blend',
    disabled: !canUseWindowMaterials,
  },
  {
    label: 'Mica',
    value: 'mica' as const,
    icon: 'i-lucide-sparkles',
    disabled: !canUseWindowMaterials,
  },
])

const appearanceDescriptions: Record<AppearanceMode, string> = {
  off: 'Use Cloudburst\'s solid app colors without a window material.',
  toned: 'Keep the standard colors with a subtle hint of the desktop material.',
  mica: 'Let the Windows desktop material show clearly through the app.',
}

const colorModeOptions = [
  { label: 'System', value: 'system', icon: 'i-lucide-monitor' },
  { label: 'Light', value: 'light', icon: 'i-lucide-sun' },
  { label: 'Dark', value: 'dark', icon: 'i-lucide-moon' },
]

const colorModeDescriptions: Record<string, string> = {
  system: 'Follow the Windows personalization setting.',
  light: 'Use Cloudburst\'s light theme.',
  dark: 'Use Cloudburst\'s dark theme.',
}

const refreshCadenceOptions: { label: string, value: RefreshCadence }[] = [
  { label: 'Fast', value: 'fast' },
  { label: 'Normal', value: 'normal' },
  { label: 'Slow', value: 'slow' },
]

const refreshCadenceDescriptions: Record<RefreshCadence, string> = {
  fast: 'Check the qBittorrent instance for transfer activity every 2 seconds.',
  normal: 'Check the qBittorrent instance for transfer activity every 5 seconds.',
  slow: 'Check the qBittorrent instance for transfer activity every 15 seconds.',
}

const placeholderDescription = computed(() => {
  if (placeholderForced) return 'Always on in the browser preview.'
  return 'Show a sample torrent list instead of the qBittorrent library — handy for screenshots and demos.'
})

const notificationDescription = canUseNotifications
  ? 'Notify when a torrent finishes downloading or encounters an error.'
  : 'Available in the Cloudburst desktop app.'
</script>

<template>
  <UModal v-model:open="open" title="Settings" description="Personalize how Cloudburst looks and behaves.">
    <template #body>
      <div class="space-y-6">
        <section class="space-y-3">
          <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Appearance</h3>
          <div class="space-y-2">
            <div>
              <p class="text-sm font-medium text-highlighted">Window material</p>
              <p class="mt-0.5 text-xs text-muted">
                {{ canUseWindowMaterials ? appearanceDescriptions[appearanceMode] : 'Toned and Mica are available in the Windows desktop app.' }}
              </p>
            </div>
            <URadioGroup
              :model-value="appearanceMode"
              :items="appearanceOptions"
              variant="table"
              orientation="horizontal"
              indicator="hidden"
              size="sm"
              aria-label="Window material"
              :ui="{ fieldset: 'grid grid-cols-3', item: 'justify-center', wrapper: 'items-center' }"
              @update:model-value="setAppearanceMode"
            />
          </div>
          <div class="space-y-2">
            <div>
              <p class="text-sm font-medium text-highlighted">Color mode</p>
              <p class="mt-0.5 text-xs text-muted">
                {{ colorModeDescriptions[colorMode.preference] ?? colorModeDescriptions.system }}
              </p>
            </div>
            <URadioGroup
              v-model="colorMode.preference"
              :items="colorModeOptions"
              variant="table"
              orientation="horizontal"
              indicator="hidden"
              size="sm"
              aria-label="Color mode"
              :ui="{ fieldset: 'grid grid-cols-3', item: 'justify-center', wrapper: 'items-center' }"
            />
          </div>
        </section>

        <section class="space-y-3">
          <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Behavior</h3>
          <div class="space-y-2">
            <div>
              <p class="text-sm font-medium text-highlighted">Refresh cadence</p>
              <p class="mt-0.5 text-xs text-muted">{{ refreshCadenceDescriptions[refreshCadence] }}</p>
            </div>
            <URadioGroup
              :model-value="refreshCadence"
              :items="refreshCadenceOptions"
              variant="table"
              orientation="horizontal"
              indicator="hidden"
              size="sm"
              aria-label="Refresh cadence"
              :ui="{ fieldset: 'grid grid-cols-3', item: 'justify-center', wrapper: 'items-center' }"
              @update:model-value="setRefreshCadence"
            />
          </div>
          <div class="flex items-start justify-between gap-3">
            <div>
              <p class="text-sm font-medium text-highlighted">Torrent notifications</p>
              <p class="mt-0.5 text-xs text-muted">{{ notificationDescription }}</p>
            </div>
            <USwitch
              :model-value="canUseNotifications && notificationsEnabled"
              :disabled="!canUseNotifications"
              aria-label="Torrent notifications"
              @update:model-value="setNotificationsEnabled"
            />
          </div>
          <div class="flex items-start justify-between gap-3">
            <div>
              <p class="text-sm font-medium text-highlighted">Placeholder torrents</p>
              <p class="mt-0.5 text-xs text-muted">{{ placeholderDescription }}</p>
            </div>
            <USwitch
              :model-value="placeholderForced || placeholderEnabled"
              :disabled="placeholderForced"
              aria-label="Placeholder torrents"
              @update:model-value="setPlaceholderEnabled"
            />
          </div>
        </section>
      </div>
    </template>
  </UModal>
</template>
