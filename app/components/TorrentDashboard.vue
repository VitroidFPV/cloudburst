<script setup lang="ts">
import type { DropdownMenuItem, NavigationMenuItem, TableColumn } from '@nuxt/ui'
import type { Torrent, TorrentAction, TorrentStatus } from '~/types/torrent'
import { formatBytes, formatSpeed, statusIcon, statusLabel } from '~/utils/torrent-format'

const toast = useToast()
const {
  visibleTorrents,
  selectedIds,
  filters,
  categories,
  activeFilter,
  activeCategory,
  transferTotals,
  chooseFilter,
  chooseCategory,
  toggleSelection,
  toggleAllVisible,
  execute,
} = useTorrentLibrary()

const UBadge = resolveComponent('UBadge')
const UButton = resolveComponent('UButton')
const UDropdownMenu = resolveComponent('UDropdownMenu')
const UIcon = resolveComponent('UIcon')
const UProgress = resolveComponent('UProgress')

const statusColor = {
  downloading: 'primary',
  seeding: 'success',
  paused: 'neutral',
  checking: 'info',
  stalled: 'warning',
  error: 'error',
} as const satisfies Record<TorrentStatus, 'primary' | 'success' | 'neutral' | 'info' | 'warning' | 'error'>

const allVisibleSelected = computed(() => visibleTorrents.value.length > 0 && visibleTorrents.value.every(torrent => selectedIds.value.includes(torrent.id)))
const someVisibleSelected = computed(() => visibleTorrents.value.some(torrent => selectedIds.value.includes(torrent.id)))

const libraryItems = computed<NavigationMenuItem[]>(() => filters.value.map(filter => ({
  label: filter.label,
  icon: filter.icon,
  badge: String(filter.count),
  active: activeFilter.value === filter.id && !activeCategory.value,
  onSelect: () => chooseFilter(filter.id),
})))

const categoryItems = computed<NavigationMenuItem[]>(() => [
  {
    label: 'All categories',
    icon: 'i-lucide-folders',
    active: !activeCategory.value,
    onSelect: () => chooseCategory(''),
  },
  ...categories.value.map(category => ({
    label: category,
    icon: 'i-lucide-folder',
    active: activeCategory.value === category,
    onSelect: () => chooseCategory(category),
  })),
])

const runAction = (action: TorrentAction, ids?: string[]) => {
  toast.add(execute(action, ids))
}

const showPlaceholder = (feature: string) => {
  toast.add({
    title: feature,
    description: 'This control is not wired in the application scaffold yet.',
    color: 'neutral',
  })
}

const rowActions = (torrent: Torrent): DropdownMenuItem[][] => [[
  { label: 'Resume', icon: 'i-lucide-play', onSelect: () => runAction('resume', [torrent.id]) },
  { label: 'Pause', icon: 'i-lucide-pause', onSelect: () => runAction('pause', [torrent.id]) },
  { label: 'Set category', icon: 'i-lucide-folder', onSelect: () => showPlaceholder('Set category') },
], [
  { label: 'Remove torrent', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => runAction('remove', [torrent.id]) },
]]

const columns: TableColumn<Torrent>[] = [
  {
    id: 'select',
  },
  {
    accessorKey: 'name',
    header: 'Torrent',
    cell: ({ row }) => h('div', { class: 'flex max-w-64 min-w-0 items-center gap-3' }, [
      h(UIcon, { name: statusIcon[row.original.status], class: 'size-4 shrink-0 text-muted' }),
      h('div', { class: 'min-w-0' }, [
        h('p', { class: 'truncate font-medium text-highlighted' }, row.original.name),
        h('p', { class: 'truncate text-xs text-muted' }, `${row.original.category} · ${formatBytes(row.original.size)}`),
      ]),
    ]),
  },
  {
    accessorKey: 'progress',
    header: 'Progress',
    cell: ({ row }) => h('div', { class: 'flex min-w-24 items-center gap-2' }, [
      h(UProgress, { modelValue: row.original.progress, size: 'xs', class: 'w-16' }),
      h('span', { class: 'w-8 text-right font-mono text-xs text-muted' }, `${row.original.progress}%`),
    ]),
  },
  {
    accessorKey: 'status',
    header: 'Status',
    cell: ({ row }) => h(UBadge, { color: statusColor[row.original.status], variant: 'subtle' }, () => statusLabel[row.original.status]),
  },
  {
    accessorKey: 'downSpeed',
    header: 'Down',
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, formatSpeed(row.original.downSpeed)),
  },
  {
    accessorKey: 'upSpeed',
    header: 'Up',
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, formatSpeed(row.original.upSpeed)),
  },
  {
    accessorKey: 'eta',
    header: 'ETA',
    cell: ({ row }) => h('span', { class: 'font-mono text-xs text-muted' }, row.original.eta),
  },
  {
    id: 'actions',
    cell: ({ row }) => h(UDropdownMenu, { items: rowActions(row.original), content: { align: 'end' } }, () => h(UButton, {
      icon: 'i-lucide-ellipsis-vertical',
      color: 'neutral',
      variant: 'ghost',
      'aria-label': `Actions for ${row.original.name}`,
    })),
  },
]
</script>

<template>
  <UDashboardGroup unit="rem" class="h-screen">
    <UDashboardSidebar id="cloudburst-sidebar" class="bg-elevated/25">
      <template #header>
        <div class="flex items-center gap-2 px-1">
          <UIcon name="i-lucide-cloud-rain" class="size-5 text-primary" />
          <span class="font-semibold text-highlighted">Cloudburst</span>
        </div>
      </template>

      <template #default>
        <div>
          <p class="mb-2 px-2 text-xs font-medium text-muted">Library</p>
          <UNavigationMenu :items="libraryItems" orientation="vertical" />
        </div>

        <div class="mt-6">
          <p class="mb-2 px-2 text-xs font-medium text-muted">Categories</p>
          <UNavigationMenu :items="categoryItems" orientation="vertical" />
        </div>
      </template>

      <template #footer>
        <div class="space-y-3 px-1 text-sm">
          <div class="flex items-center justify-between gap-2">
            <span class="flex items-center gap-2 text-muted"><span class="size-2 rounded-full bg-warning" />qBittorrent</span>
            <UBadge label="Demo data" color="neutral" variant="outline" size="sm" />
          </div>
          <div class="flex items-center justify-between font-mono text-xs text-muted">
            <span>↓ {{ formatSpeed(transferTotals.down) }}</span>
            <span>↑ {{ formatSpeed(transferTotals.up) }}</span>
          </div>
        </div>
      </template>
    </UDashboardSidebar>

    <UDashboardPanel id="torrent-list">
      <template #body>
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex min-w-0 flex-wrap items-center gap-2">
            <div class="flex shrink-0 items-center gap-2">
              <h1 class="text-2xl font-semibold text-highlighted">Torrents</h1>
              <UBadge label="Placeholder" color="neutral" variant="subtle" />
            </div>

            <USeparator orientation="vertical" class="mx-2 h-6" />

            <div v-if="selectedIds.length" class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-sm text-muted">{{ selectedIds.length }} selected</span>
              <UButton label="Resume" icon="i-lucide-play" color="neutral" variant="ghost" size="sm" @click="runAction('resume')" />
              <UButton label="Pause" icon="i-lucide-pause" color="neutral" variant="ghost" size="sm" @click="runAction('pause')" />
              <UButton label="Category" icon="i-lucide-folder" color="neutral" variant="ghost" size="sm" @click="showPlaceholder('Set category')" />
              <UButton label="Remove" icon="i-lucide-trash-2" color="error" variant="ghost" size="sm" @click="runAction('remove')" />
            </div>
            <p v-else class="text-sm text-muted">{{ visibleTorrents.length }} torrents</p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <UButton label="Add torrent" icon="i-lucide-plus" @click="showPlaceholder('Add torrent')" />
            <UButton label="Columns" icon="i-lucide-columns-3" color="neutral" variant="outline" size="sm" @click="showPlaceholder('Choose columns')" />
            <UButton icon="i-lucide-settings" color="neutral" variant="outline" aria-label="Settings" @click="showPlaceholder('Settings')" />
          </div>
        </div>

        <UTable :data="visibleTorrents" :columns="columns" class="min-h-0 flex-1">
          <template #select-header>
            <UCheckbox
              :model-value="allVisibleSelected ? true : someVisibleSelected ? 'indeterminate' : false"
              aria-label="Select all visible torrents"
              @change="toggleAllVisible"
            />
          </template>

          <template #select-cell="{ row }">
            <UCheckbox
              :model-value="selectedIds.includes(row.original.id)"
              :aria-label="`Select ${row.original.name}`"
              @change="toggleSelection(row.original.id)"
            />
          </template>
        </UTable>

        <div v-if="!visibleTorrents.length" class="grid min-h-48 place-items-center text-center">
          <div>
            <UIcon name="i-lucide-list-filter" class="mx-auto size-8 text-muted" />
            <p class="mt-3 font-medium text-highlighted">No torrents in this view</p>
            <p class="mt-1 text-sm text-muted">Choose another filter from the sidebar.</p>
          </div>
        </div>
      </template>
    </UDashboardPanel>
  </UDashboardGroup>
</template>
