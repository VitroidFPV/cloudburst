<script setup lang="ts">
import type { AppearanceMode } from '~/composables/useAppearanceSetting'

const { appearanceMode, canUseWindowMaterials, setAppearanceMode } = useAppearanceSetting()

const open = defineModel<boolean>('open', { default: false })

const appearanceOptions = computed(() => [
  {
    label: 'Off',
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
  off: 'Use Cloudburst\'s standard, fully opaque app colors.',
  toned: 'Keep the standard colors with a subtle hint of the desktop material.',
  mica: 'Let the Windows desktop material show clearly through the app.',
}
</script>

<template>
  <UModal v-model:open="open" title="Settings" description="Personalize how Cloudburst looks and behaves.">
    <template #body>
      <div class="space-y-3">
        <section class="space-y-2">
          <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Appearance</h3>
          <div class="space-y-2.5 rounded-lg border border-default bg-elevated/25 p-3">
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
        </section>
      </div>
    </template>
  </UModal>
</template>
