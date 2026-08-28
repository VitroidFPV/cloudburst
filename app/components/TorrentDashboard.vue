<script setup lang="ts">
import type { NavigationMenuItem, TableColumn } from '@nuxt/ui'
import type { AuthenticationMode, ConnectionInput, Torrent, TorrentStatus } from '~/types/torrent'
import { formatAddedOn, formatAddedOnFull, formatBytes, formatEta, formatSpeed, statusIcon, statusLabel } from '~/utils/torrent-format'

const toast = useToast()
const {
  torrents,
  visibleTorrents,
  filters,
  categories,
  activeFilter,
  activeCategory,
  transferTotals,
  connectionStatus,
  connectionError,
  connectionEndpoint,
  connectionVersion,
  savedProfile,
  stale,
  refreshing,
  connect,
  restoreSavedConnection,
  refresh,
  disconnect,
  chooseFilter,
  chooseCategory,
} = useTorrentLibrary()

const UBadge = resolveComponent('UBadge')
const UIcon = resolveComponent('UIcon')
const UProgress = resolveComponent('UProgress')

const defaultColumnSizing = {
  name: 320,
  progress: 140,
  status: 130,
  downSpeed: 110,
  upSpeed: 110,
  etaSeconds: 100,
  ratio: 90,
  seeds: 80,
  peers: 80,
  addedOn: 100,
  tags: 140,
}
const hideableColumns = [
  { id: 'progress', label: 'Progress' },
  { id: 'status', label: 'Status' },
  { id: 'downSpeed', label: 'Down speed' },
  { id: 'upSpeed', label: 'Up speed' },
  { id: 'etaSeconds', label: 'ETA' },
  { id: 'ratio', label: 'Ratio' },
  { id: 'seeds', label: 'Seeds' },
  { id: 'peers', label: 'Peers' },
  { id: 'addedOn', label: 'Added on' },
  { id: 'tags', label: 'Tags' },
]

const defaultColumnVisibility: Record<string, boolean> = { ratio: false, seeds: false, peers: false, tags: false }
const columnVisibility = ref<Record<string, boolean>>({ ...defaultColumnVisibility })
const columnVisibilityStorageKey = 'cloudburst:torrent-column-visibility'

const isColumnVisible = (columnId: string) => columnVisibility.value[columnId] !== false

const toggleColumnVisibility = (columnId: string) => {
  columnVisibility.value = { ...columnVisibility.value, [columnId]: !isColumnVisible(columnId) }
}

const columnVisibilityItems = computed(() => hideableColumns.map(column => ({
  label: column.label,
  type: 'checkbox' as const,
  checked: isColumnVisible(column.id),
  onSelect: (event: Event) => {
    event.preventDefault()
    toggleColumnVisibility(column.id)
  },
})))

const columnSizing = ref<Record<string, number>>({ ...defaultColumnSizing })
const columnSizingStorageKey = 'cloudburst:torrent-column-sizing'
const tableWidth = computed(() => Object.entries(columnSizing.value)
  .filter(([column]) => isColumnVisible(column))
  .reduce((total, [, size]) => total + size, 0))

interface ResizableHeaderContext {
  header: {
    column: {
      id: string
      columnDef: { minSize?: number, maxSize?: number }
      getSize: () => number
      resetSize: () => void
    }
  }
}

interface ResizableColumnContext {
  column: {
    getSize: () => number
  }
}

const clampColumnSize = (column: ResizableHeaderContext['header']['column'], requestedSize: number) => {
  const minimum = column.columnDef.minSize ?? 20
  const maximum = column.columnDef.maxSize ?? Number.MAX_SAFE_INTEGER
  return Math.min(maximum, Math.max(minimum, requestedSize))
}

const resizableColumnStyle = ({ column }: ResizableColumnContext) => ({
  width: `${column.getSize()}px`,
})

const resizableColumnMeta = {
  class: {
    th: 'column-resize-highlight column-resize-highlight-header',
    td: 'column-resize-highlight column-resize-highlight-cell',
  },
  style: {
    th: resizableColumnStyle,
    td: resizableColumnStyle,
  },
}

let cancelActiveColumnResize: (() => void) | undefined

const startColumnResize = (event: PointerEvent, column: ResizableHeaderContext['header']['column']) => {
  if (!event.isPrimary || event.button !== 0) return

  cancelActiveColumnResize?.()

  const handle = event.currentTarget as HTMLElement
  const headerCell = handle.closest('th') as HTMLTableCellElement | null
  const tableRoot = handle.closest('.torrent-table')
  if (!headerCell || !tableRoot) return

  const columnIndex = headerCell.cellIndex
  const highlightedCells = [
    headerCell,
    ...Array.from(tableRoot.querySelectorAll<HTMLTableCellElement>('tbody td'))
      .filter(cell => cell.cellIndex === columnIndex && cell.colSpan === 1),
  ]
  const startX = event.clientX
  const startSize = column.getSize()
  let nextSize = startSize
  let animationFrame: number | undefined
  let finished = false

  handle.dataset.resizing = 'true'
  highlightedCells.forEach((cell) => {
    cell.style.setProperty('--column-resize-highlight-opacity', '1')
  })

  const paintPreview = () => {
    const delta = nextSize - startSize
    handle.style.transform = `translateX(${delta}px)`
    highlightedCells.forEach((cell) => {
      cell.style.setProperty('--column-resize-highlight-width', `calc(100% + ${delta}px)`)
    })
    animationFrame = undefined
  }

  const updatePreview = (clientX: number) => {
    nextSize = clampColumnSize(column, startSize + clientX - startX)
    if (animationFrame === undefined) animationFrame = requestAnimationFrame(paintPreview)
  }

  const cleanup = (commit: boolean, clientX?: number) => {
    if (finished) return
    finished = true
    if (clientX !== undefined) updatePreview(clientX)
    if (animationFrame !== undefined) cancelAnimationFrame(animationFrame)
    paintPreview()

    document.removeEventListener('pointermove', handlePointerMove)
    document.removeEventListener('pointerup', handlePointerUp)
    document.removeEventListener('pointercancel', handlePointerCancel)
    delete handle.dataset.resizing
    handle.style.transform = ''
    highlightedCells.forEach((cell) => {
      cell.style.setProperty('--column-resize-highlight-opacity', '0')
      cell.style.setProperty('--column-resize-highlight-width', '100%')
    })

    if (commit && nextSize !== startSize) {
      columnSizing.value = { ...columnSizing.value, [column.id]: nextSize }
    }
    cancelActiveColumnResize = undefined
  }

  const handlePointerMove = (moveEvent: PointerEvent) => {
    if (moveEvent.pointerId !== event.pointerId) return
    if (moveEvent.cancelable) moveEvent.preventDefault()
    updatePreview(moveEvent.clientX)
  }
  const handlePointerUp = (upEvent: PointerEvent) => {
    if (upEvent.pointerId === event.pointerId) cleanup(true, upEvent.clientX)
  }
  const handlePointerCancel = (cancelEvent: PointerEvent) => {
    if (cancelEvent.pointerId === event.pointerId) cleanup(false)
  }

  document.addEventListener('pointermove', handlePointerMove)
  document.addEventListener('pointerup', handlePointerUp)
  document.addEventListener('pointercancel', handlePointerCancel)
  cancelActiveColumnResize = () => cleanup(false)
}

const resizeColumnWithKeyboard = (event: KeyboardEvent, column: ResizableHeaderContext['header']['column']) => {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return

  event.preventDefault()
  const delta = event.key === 'ArrowLeft' ? -16 : 16
  const minimum = column.columnDef.minSize ?? 20
  const maximum = column.columnDef.maxSize ?? Number.MAX_SAFE_INTEGER
  columnSizing.value = {
    ...columnSizing.value,
    [column.id]: Math.min(maximum, Math.max(minimum, column.getSize() + delta)),
  }
}

const resizableHeader = (label: string) => ({ header }: ResizableHeaderContext) => h('div', { class: 'relative flex items-center pe-2' }, [
  h('span', label),
  h('div', {
    role: 'separator',
    tabindex: 0,
    'aria-label': `Resize ${label} column`,
    'aria-orientation': 'vertical',
    'aria-valuemin': header.column.columnDef.minSize,
    'aria-valuemax': header.column.columnDef.maxSize,
    'aria-valuenow': header.column.getSize(),
    class: [
      'absolute -inset-y-3.5 -right-6 z-10 w-4 cursor-col-resize touch-none select-none outline-none',
      'after:absolute after:inset-y-2 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-accented after:transition-colors',
      'hover:after:bg-primary focus-visible:after:w-0.5 focus-visible:after:bg-primary',
      'data-[resizing=true]:after:w-0.5 data-[resizing=true]:after:bg-primary',
    ],
    onPointerdown: (event: PointerEvent) => startColumnResize(event, header.column),
    onDblclick: () => header.column.resetSize(),
    onKeydown: (event: KeyboardEvent) => resizeColumnWithKeyboard(event, header.column),
  }),
])

const settingsOpen = ref(false)
const authenticationMode = ref<AuthenticationMode>('apiKey')
const connectionForm = reactive({
  endpoint: 'http://localhost:8080',
  apiKey: '',
  username: '',
  password: '',
})

const statusColor = {
  downloading: 'primary',
  seeding: 'success',
  paused: 'neutral',
  checking: 'info',
  stalled: 'warning',
  error: 'error',
} as const satisfies Record<TorrentStatus, 'primary' | 'success' | 'neutral' | 'info' | 'warning' | 'error'>

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

const connectionBadge = computed(() => {
  if (stale.value) return { label: 'Stale', color: 'warning' as const }
  if (connectionStatus.value === 'connected') return { label: `v${connectionVersion.value}`, color: 'success' as const }
  if (connectionStatus.value === 'connecting') return { label: 'Connecting', color: 'neutral' as const }
  return { label: 'Disconnected', color: 'error' as const }
})

const connectionDotClass = computed(() => {
  if (stale.value) return 'bg-warning'
  if (connectionStatus.value === 'connected') return 'bg-success'
  if (connectionStatus.value === 'connecting') return 'animate-pulse bg-muted'
  return 'bg-error'
})

const canReuseSavedCredential = computed(() => {
  if (!savedProfile.value) return false

  const currentEndpoint = connectionForm.endpoint.replace(/\/+$/, '')
  const sameUsername = authenticationMode.value !== 'credentials'
    || savedProfile.value.username === connectionForm.username

  return savedProfile.value.endpoint === currentEndpoint
    && savedProfile.value.authenticationMode === authenticationMode.value
    && sameUsername
})

watch(savedProfile, (profile) => {
  if (!profile) return
  connectionForm.endpoint = profile.endpoint
  authenticationMode.value = profile.authenticationMode
  connectionForm.username = profile.username || ''
  connectionForm.apiKey = ''
  connectionForm.password = ''
}, { immediate: true })

const submitConnection = async () => {
  if (!('__TAURI_INTERNALS__' in window)) {
    connectionError.value = 'qBittorrent connections are available in the Cloudburst desktop app.'
    return
  }

  const input: ConnectionInput = authenticationMode.value === 'apiKey'
    ? { endpoint: connectionForm.endpoint, authenticationMode: 'apiKey', apiKey: connectionForm.apiKey }
    : {
        endpoint: connectionForm.endpoint,
        authenticationMode: 'credentials',
        username: connectionForm.username,
        password: connectionForm.password,
      }

  if (await connect(input)) {
    settingsOpen.value = false
    toast.add({
      title: 'qBittorrent connected',
      description: `${torrents.value.length} torrent${torrents.value.length === 1 ? '' : 's'} loaded from ${connectionEndpoint.value}.`,
      color: 'success',
    })
  }
}

const retryConnection = async () => {
  if (!connectionEndpoint.value) {
    settingsOpen.value = true
    return
  }

  const restored = connectionStatus.value === 'disconnected' && savedProfile.value
    ? await restoreSavedConnection()
    : await refresh()

  if (restored) {
    toast.add({ title: 'Connection restored', description: 'The torrent list is current again.', color: 'success' })
  }
}

const disconnectConnection = async () => {
  if (await disconnect()) {
    settingsOpen.value = false
    toast.add({ title: 'qBittorrent connection forgotten', description: 'The saved profile and protected credential were removed.', color: 'neutral' })
  }
}

const columns: TableColumn<Torrent>[] = [
  {
    accessorKey: 'name',
    header: resizableHeader('Torrent'),
    size: defaultColumnSizing.name,
    minSize: 180,
    maxSize: 720,
    enableHiding: false,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('div', { class: 'flex w-full min-w-0 items-center gap-3' }, [
      h(UIcon, { name: statusIcon[row.original.status], class: 'size-4 shrink-0 text-muted' }),
      h('div', { class: 'min-w-0' }, [
        h('p', { class: 'truncate font-medium text-highlighted' }, row.original.name),
        h('p', { class: 'truncate text-xs text-muted' }, `${row.original.category || 'Uncategorized'} · ${formatBytes(row.original.size)}`),
      ]),
    ]),
  },
  {
    accessorKey: 'progress',
    header: resizableHeader('Progress'),
    size: defaultColumnSizing.progress,
    minSize: 112,
    maxSize: 260,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('div', { class: 'flex min-w-24 items-center gap-2' }, [
      h(UProgress, { modelValue: row.original.progress, size: 'xs', class: 'min-w-0 flex-1' }),
      h('span', { class: 'w-10 text-right font-mono text-xs text-muted' }, `${row.original.progress}%`),
    ]),
  },
  {
    accessorKey: 'status',
    header: resizableHeader('Status'),
    size: defaultColumnSizing.status,
    minSize: 96,
    maxSize: 220,
    meta: resizableColumnMeta,
    cell: ({ row }) => h(UBadge, { color: statusColor[row.original.status], variant: 'subtle' }, () => statusLabel[row.original.status]),
  },
  {
    accessorKey: 'downSpeed',
    header: resizableHeader('Down'),
    size: defaultColumnSizing.downSpeed,
    minSize: 88,
    maxSize: 200,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, formatSpeed(row.original.downSpeed)),
  },
  {
    accessorKey: 'upSpeed',
    header: resizableHeader('Up'),
    size: defaultColumnSizing.upSpeed,
    minSize: 88,
    maxSize: 200,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, formatSpeed(row.original.upSpeed)),
  },
  {
    accessorKey: 'etaSeconds',
    header: resizableHeader('ETA'),
    size: defaultColumnSizing.etaSeconds,
    minSize: 80,
    maxSize: 180,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs text-muted' }, formatEta(row.original.etaSeconds, row.original.status)),
  },
  {
    accessorKey: 'ratio',
    header: resizableHeader('Ratio'),
    size: defaultColumnSizing.ratio,
    minSize: 70,
    maxSize: 160,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, row.original.ratio.toFixed(2)),
  },
  {
    accessorKey: 'seeds',
    header: resizableHeader('Seeds'),
    size: defaultColumnSizing.seeds,
    minSize: 64,
    maxSize: 160,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, String(row.original.seeds)),
  },
  {
    accessorKey: 'peers',
    header: resizableHeader('Peers'),
    size: defaultColumnSizing.peers,
    minSize: 64,
    maxSize: 160,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs tabular-nums' }, String(row.original.peers)),
  },
  {
    accessorKey: 'addedOn',
    header: resizableHeader('Added'),
    size: defaultColumnSizing.addedOn,
    minSize: 80,
    maxSize: 200,
    meta: resizableColumnMeta,
    cell: ({ row }) => h('span', { class: 'font-mono text-xs text-muted', title: formatAddedOnFull(row.original.addedOn) }, formatAddedOn(row.original.addedOn)),
  },
  {
    accessorKey: 'tags',
    header: resizableHeader('Tags'),
    size: defaultColumnSizing.tags,
    minSize: 90,
    maxSize: 280,
    meta: resizableColumnMeta,
    cell: ({ row }) => row.original.tags.length
      ? h('div', { class: 'flex min-w-0 items-center gap-1 overflow-hidden' }, row.original.tags.map(tag => h(UBadge, { color: 'neutral', variant: 'subtle', size: 'sm', class: 'max-w-full shrink-0 truncate' }, () => tag)))
      : h('span', { class: 'text-xs text-muted' }, '—'),
  },
  {
    id: 'layoutSpacer',
    header: () => null,
    cell: () => null,
    enableResizing: false,
    enableHiding: false,
    meta: {
      class: {
        th: 'p-0',
        td: 'p-0',
      },
    },
  },
]

let refreshTimer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  try {
    const savedSizing = JSON.parse(localStorage.getItem(columnSizingStorageKey) || '{}') as Record<string, unknown>
    const validSizing = Object.fromEntries(Object.entries(savedSizing).filter(([column, size]) => (
      column in defaultColumnSizing && typeof size === 'number' && Number.isFinite(size)
    ))) as Record<string, number>
    columnSizing.value = { ...defaultColumnSizing, ...validSizing }

    const savedVisibility = JSON.parse(localStorage.getItem(columnVisibilityStorageKey) || '{}') as Record<string, unknown>
    const validVisibility = Object.fromEntries(Object.entries(savedVisibility).filter(([column, visible]) => (
      hideableColumns.some(({ id }) => id === column) && typeof visible === 'boolean'
    ))) as Record<string, boolean>
    columnVisibility.value = { ...defaultColumnVisibility, ...validVisibility }
  } catch {
    localStorage.removeItem(columnSizingStorageKey)
    localStorage.removeItem(columnVisibilityStorageKey)
  }

  void restoreSavedConnection()
  refreshTimer = setInterval(() => {
    if (connectionStatus.value === 'connected') void refresh()
  }, 5_000)
})

onBeforeUnmount(() => {
  cancelActiveColumnResize?.()
  if (refreshTimer) clearInterval(refreshTimer)
})

watch(columnSizing, sizing => localStorage.setItem(columnSizingStorageKey, JSON.stringify(sizing)), { deep: true })
watch(columnVisibility, visibility => localStorage.setItem(columnVisibilityStorageKey, JSON.stringify(visibility)), { deep: true })
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
        <button type="button" class="w-full space-y-3 rounded-md px-1 py-1 text-left text-sm hover:bg-elevated" @click="settingsOpen = true">
          <span class="flex items-center justify-between gap-2">
            <span class="flex min-w-0 items-center gap-2 text-muted">
              <span class="size-2 shrink-0 rounded-full" :class="connectionDotClass" />
              <span class="truncate">qBittorrent</span>
            </span>
            <UBadge :label="connectionBadge.label" :color="connectionBadge.color" variant="outline" size="sm" />
          </span>
          <span class="flex items-center justify-between font-mono text-xs text-muted">
            <span>↓ {{ formatSpeed(transferTotals.down) }}</span>
            <span>↑ {{ formatSpeed(transferTotals.up) }}</span>
          </span>
        </button>
      </template>
    </UDashboardSidebar>

    <UDashboardPanel id="torrent-list">
      <template #body>
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex min-w-0 items-center gap-2">
            <h1 class="text-2xl font-semibold text-highlighted">Torrents</h1>
            <UBadge label="Read only" color="neutral" variant="subtle" />
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <UDropdownMenu :items="columnVisibilityItems" :content="{ align: 'end' }">
              <UButton
                label="Columns"
                icon="i-lucide-columns-2"
                color="neutral"
                variant="outline"
                size="sm"
                aria-label="Choose visible columns"
              />
            </UDropdownMenu>
            <UButton
              v-if="connectionEndpoint"
              icon="i-lucide-refresh-cw"
              color="neutral"
              variant="outline"
              size="sm"
              aria-label="Refresh torrents"
              :loading="refreshing"
              @click="retryConnection"
            />
            <UButton
              :label="connectionEndpoint ? 'Connection' : 'Connect qBittorrent'"
              :icon="connectionEndpoint ? 'i-lucide-settings' : 'i-lucide-plug-zap'"
              :color="connectionEndpoint ? 'neutral' : 'primary'"
              :variant="connectionEndpoint ? 'outline' : 'solid'"
              size="sm"
              @click="settingsOpen = true"
            />
          </div>
        </div>

        <div v-if="stale" class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-warning/35 bg-warning/10 px-4 py-3 text-sm">
          <div class="flex min-w-0 items-start gap-3">
            <UIcon name="i-lucide-cloud-off" class="mt-0.5 size-4 shrink-0 text-warning" />
            <div>
              <p class="font-medium text-highlighted">Disconnected — showing stale torrents</p>
              <p class="mt-0.5 text-muted">{{ connectionError }}</p>
            </div>
          </div>
          <UButton label="Reconnect" icon="i-lucide-refresh-cw" color="warning" variant="soft" size="sm" :loading="refreshing" @click="retryConnection" />
        </div>

        <UTable
          v-if="visibleTorrents.length"
          v-model:column-sizing="columnSizing"
          v-model:column-visibility="columnVisibility"
          :data="visibleTorrents"
          :columns="columns"
          :column-sizing-options="{ columnResizeMode: 'onEnd' }"
          :virtualize="{ estimateSize: 65, overscan: 8 }"
          :style="{ '--torrent-table-width': `${tableWidth}px` }"
          :ui="{ base: 'min-w-0', th: 'relative overflow-visible' }"
          class="torrent-table min-h-0 flex-1"
        />

        <div v-else class="grid min-h-64 flex-1 place-items-center text-center">
          <div v-if="!torrents.length && connectionStatus === 'disconnected'" class="max-w-md">
            <UIcon :name="connectionError ? 'i-lucide-circle-alert' : 'i-lucide-plug-zap'" class="mx-auto size-9 text-muted" />
            <p class="mt-3 font-medium text-highlighted">{{ connectionError ? 'Could not connect to qBittorrent' : 'Connect a qBittorrent instance' }}</p>
            <p class="mt-1 text-sm text-muted">{{ connectionError || 'Cloudburst reads torrents through the qBittorrent 5.2+ Web API.' }}</p>
            <UButton class="mt-4" :label="connectionError ? 'Connection settings' : 'Connect qBittorrent'" icon="i-lucide-plug-zap" @click="settingsOpen = true" />
          </div>
          <div v-else-if="connectionStatus === 'connecting'">
            <UIcon name="i-lucide-loader-circle" class="mx-auto size-9 animate-spin text-muted" />
            <p class="mt-3 font-medium text-highlighted">Connecting to qBittorrent</p>
            <p class="mt-1 text-sm text-muted">Checking authentication and compatibility…</p>
          </div>
          <div v-else-if="!torrents.length">
            <UIcon name="i-lucide-inbox" class="mx-auto size-9 text-muted" />
            <p class="mt-3 font-medium text-highlighted">No torrents in qBittorrent</p>
            <p class="mt-1 text-sm text-muted">The connection is healthy and the torrent library is empty.</p>
          </div>
          <div v-else>
            <UIcon name="i-lucide-list-filter" class="mx-auto size-8 text-muted" />
            <p class="mt-3 font-medium text-highlighted">No torrents in this view</p>
            <p class="mt-1 text-sm text-muted">Choose another filter from the sidebar.</p>
          </div>
        </div>
      </template>
    </UDashboardPanel>
  </UDashboardGroup>

  <UModal v-model:open="settingsOpen" title="qBittorrent connection" description="Connect directly to the qBittorrent 5.2+ Web API.">
    <template #body>
      <form id="qbittorrent-connection-form" class="space-y-5" @submit.prevent="submitConnection">
        <UFormField label="WebUI URL" description="Use the URL configured in qBittorrent's WebUI preferences." required>
          <UInput v-model="connectionForm.endpoint" class="w-full" placeholder="http://localhost:8080" autocomplete="url" />
        </UFormField>

        <UFormField label="Authentication">
          <div class="grid grid-cols-2 gap-2">
            <UButton
              type="button"
              label="API key"
              icon="i-lucide-key-round"
              :variant="authenticationMode === 'apiKey' ? 'solid' : 'outline'"
              :color="authenticationMode === 'apiKey' ? 'primary' : 'neutral'"
              block
              @click="authenticationMode = 'apiKey'"
            />
            <UButton
              type="button"
              label="Username & password"
              icon="i-lucide-user-round"
              :variant="authenticationMode === 'credentials' ? 'solid' : 'outline'"
              :color="authenticationMode === 'credentials' ? 'primary' : 'neutral'"
              block
              @click="authenticationMode = 'credentials'"
            />
          </div>
        </UFormField>

        <UFormField v-if="authenticationMode === 'apiKey'" label="API key" :description="canReuseSavedCredential ? 'Leave blank to reuse the protected API key.' : 'Generate an API key in qBittorrent\'s WebUI preferences.'" :required="!canReuseSavedCredential">
          <UInput v-model="connectionForm.apiKey" class="w-full" type="password" placeholder="qbt_…" autocomplete="off" />
        </UFormField>

        <div v-else class="grid gap-4 sm:grid-cols-2">
          <UFormField label="Username" required>
            <UInput v-model="connectionForm.username" class="w-full" autocomplete="username" />
          </UFormField>
          <UFormField label="Password" :description="canReuseSavedCredential ? 'Leave blank to reuse the protected password.' : undefined" :required="!canReuseSavedCredential">
            <UInput v-model="connectionForm.password" class="w-full" type="password" autocomplete="current-password" />
          </UFormField>
        </div>

        <div v-if="connectionError" class="flex items-start gap-2 rounded-md border border-error/30 bg-error/10 px-3 py-2.5 text-sm text-error">
          <UIcon name="i-lucide-circle-alert" class="mt-0.5 size-4 shrink-0" />
          <span>{{ connectionError }}</span>
        </div>

        <p class="text-xs text-muted">The credential is saved in the operating system vault. The profile stores only the URL, authentication mode, and username.</p>
      </form>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-between gap-3">
        <UButton v-if="savedProfile" type="button" label="Forget connection" color="error" variant="ghost" @click="disconnectConnection" />
        <span v-else />
        <div class="flex gap-2">
          <UButton type="button" label="Cancel" color="neutral" variant="ghost" @click="settingsOpen = false" />
          <UButton type="submit" form="qbittorrent-connection-form" label="Connect" icon="i-lucide-plug-zap" :loading="connectionStatus === 'connecting'" />
        </div>
      </div>
    </template>
  </UModal>
</template>

<style>
.torrent-table table {
  width: 100%;
  min-width: var(--torrent-table-width);
  table-layout: fixed;
}

.torrent-table .column-resize-highlight {
  --column-resize-highlight-width: 100%;
  --column-resize-highlight-opacity: 0;

  position: relative;
}

.torrent-table .column-resize-highlight::before {
  position: absolute;
  inset: 0 auto 0 0;
  width: var(--column-resize-highlight-width);
  pointer-events: none;
  content: '';
  opacity: var(--column-resize-highlight-opacity);
  transition: opacity 150ms ease;
}

.torrent-table .column-resize-highlight-header::before {
  background: color-mix(in oklab, var(--ui-primary) 10%, transparent);
}

.torrent-table .column-resize-highlight-cell::before {
  background: color-mix(in oklab, var(--ui-primary) 5%, transparent);
}
</style>
