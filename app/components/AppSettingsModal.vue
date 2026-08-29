<script setup lang="ts">
const { micaEnabled, canUseMica, setMicaEnabled } = useMicaSetting()

const open = defineModel<boolean>('open', { default: false })
</script>

<template>
  <UModal v-model:open="open" title="Settings" description="Personalize how Cloudburst looks and behaves.">
    <template #body>
      <div class="space-y-3">
        <section class="space-y-2">
          <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Appearance</h3>
          <div class="flex items-center justify-between gap-4 rounded-lg border border-default bg-elevated/25 px-3 py-3">
            <div class="min-w-0">
              <p class="text-sm font-medium text-highlighted">Mica window material</p>
              <p class="mt-0.5 text-xs text-muted">
                {{ canUseMica ? 'Let the Windows desktop material show through app surfaces.' : 'Available in the Windows desktop app.' }}
              </p>
            </div>
            <USwitch
              :model-value="micaEnabled"
              :disabled="!canUseMica"
              aria-label="Mica window material"
              @update:model-value="setMicaEnabled"
            />
          </div>
        </section>
      </div>
    </template>
  </UModal>
</template>
