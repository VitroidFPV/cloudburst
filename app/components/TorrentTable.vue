<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from '@nuxt/ui'
import type { Torrent, TorrentStatus } from '~/types/torrent'
import { formatAddedOn, formatAddedOnFull, formatBytes, formatEta, formatSpeed, statusIcon, statusLabel } from '~/utils/torrent-format'
import { resolveTorrentSelection, shouldSelectAllTorrents } from '~/utils/torrent-selection'

const props = defineProps<{
  torrents: Torrent[]
  actionsDisabled?: boolean
  actionPending?: boolean
  autoSelectIds?: string[]
}>()

const emit = defineEmits<{
  'set-paused': [torrentIds: string[], paused: boolean]
  'remove-torrents': [torrentIds: string[], deleteFiles: boolean]
}>()

const UBadge = resolveComponent('UBadge')
const UIcon = resolveComponent('UIcon')
const UProgress = resolveComponent('UProgress')
const UButton = resolveComponent('UButton')
const UCheckbox = resolveComponent('UCheckbox')

const defaultColumnSizing = {
  select: 44,
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
const sorting = ref<Array<{ id: string, desc: boolean }>>([])
const sortingStorageKey = 'cloudburst:torrent-column-sorting'
const rowSelection = ref<Record<string, boolean>>({})
const selectionAnchorId = ref<string>()

interface TorrentTableInstance {
  tableApi: {
    getRowModel: () => { rows: TableRow<Torrent>[] }
  }
}

const torrentTable = useTemplateRef<TorrentTableInstance>('torrentTable')

const selectedTorrentIds = computed(() => Object.entries(rowSelection.value)
  .filter(([, selected]) => selected)
  .map(([torrentId]) => torrentId))
const selectedTorrents = computed(() => {
  const torrentsById = new Map(props.torrents.map(torrent => [torrent.id, torrent]))
  return selectedTorrentIds.value.flatMap(torrentId => torrentsById.get(torrentId) || [])
})
const selectedCount = computed(() => selectedTorrents.value.length)
const canStartSelected = computed(() => selectedTorrents.value.some(torrent => torrent.status === 'paused'))
const canStopSelected = computed(() => selectedTorrents.value.some(torrent => torrent.status !== 'paused'))
const activityActionsDisabled = computed(() => props.actionsDisabled || props.actionPending)

const emitSelectedActivity = (paused: boolean) => {
  if (!selectedCount.value || activityActionsDisabled.value) return
  emit('set-paused', selectedTorrents.value.map(torrent => torrent.id), paused)
}

const removeOpen = ref(false)
const removeTitle = computed(() => `Remove ${selectedCount.value === 1 ? 'torrent' : `${selectedCount.value} torrents`}`)

const confirmRemoval = (deleteFiles: boolean) => {
  if (!selectedCount.value || activityActionsDisabled.value) return
  removeOpen.value = false
  emit('remove-torrents', selectedTorrents.value.map(torrent => torrent.id), deleteFiles)
}

const contextMenuItems = computed<ContextMenuItem[][]>(() => [[
  {
    label: 'Start',
    icon: 'i-lucide-play',
    disabled: activityActionsDisabled.value || !canStartSelected.value,
    onSelect: () => emitSelectedActivity(false),
  },
  {
    label: 'Stop',
    icon: 'i-lucide-square',
    disabled: activityActionsDisabled.value || !canStopSelected.value,
    onSelect: () => emitSelectedActivity(true),
  },
], [
  {
    label: 'Remove…',
    icon: 'i-lucide-trash-2',
    disabled: activityActionsDisabled.value,
    onSelect: () => {
      removeOpen.value = true
    },
  },
]])

const selectRow = (event: Event, row: TableRow<Torrent>) => {
  const mouseEvent = event as MouseEvent
  const additive = mouseEvent.ctrlKey || mouseEvent.metaKey
  const result = resolveTorrentSelection({
    orderedIds: (torrentTable.value?.tableApi.getRowModel().rows || []).map(orderedRow => orderedRow.id),
    targetId: row.id,
    selected: rowSelection.value,
    anchorId: selectionAnchorId.value,
    additive,
    range: mouseEvent.shiftKey,
  })

  rowSelection.value = result.selected
  selectionAnchorId.value = result.anchorId
}

const prepareRowContextMenu = (_event: Event, row: TableRow<Torrent>) => {
  if (!row.getIsSelected()) {
    rowSelection.value = { [row.id]: true }
    selectionAnchorId.value = row.id
  }
}

const resetSelectionAnchor = () => {
  selectionAnchorId.value = undefined
}

const toggleRow = (row: TableRow<Torrent>, selected: boolean) => {
  selectionAnchorId.value = row.id
  row.toggleSelected(selected)
}

const isSortingEntry = (entry: unknown): entry is { id: string, desc: boolean } => {
  if (typeof entry !== 'object' || entry === null) return false
  const { id, desc } = entry as Record<string, unknown>
  return typeof id === 'string' && id in defaultColumnSizing && typeof desc === 'boolean'
}

const sortByStatusLabel = (a: { original: Torrent }, b: { original: Torrent }) => statusLabel[a.original.status].localeCompare(statusLabel[b.original.status])
const sortByEtaSeconds = (a: { original: Torrent }, b: { original: Torrent }) => (a.original.etaSeconds ?? Number.POSITIVE_INFINITY) - (b.original.etaSeconds ?? Number.POSITIVE_INFINITY)
const sortByTags = (a: { original: Torrent }, b: { original: Torrent }) => a.original.tags.join(', ').localeCompare(b.original.tags.join(', '))

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
      getIsSorted: () => false | 'asc' | 'desc'
      toggleSorting: (desc?: boolean) => void
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

const resizableHeader = (label: string) => ({ header }: ResizableHeaderContext) => {
  const isSorted = header.column.getIsSorted()

  return h('div', { class: 'relative flex items-center pe-2' }, [
    h(UButton, {
      color: 'neutral',
      variant: 'ghost',
      size: 'sm',
      label,
      icon: isSorted ? (isSorted === 'asc' ? 'i-lucide-arrow-up-narrow-wide' : 'i-lucide-arrow-down-wide-narrow') : 'i-lucide-arrow-up-down',
      class: '-ms-2.5',
      'aria-label': `Sort by ${label}`,
      onClick: () => header.column.toggleSorting(header.column.getIsSorted() === 'asc'),
    }),
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
}

const statusColor = {
  downloading: 'success',
  seeding: 'info',
  paused: 'neutral',
  checking: 'primary',
  stalled: 'warning',
  error: 'error',
} as const satisfies Record<TorrentStatus, 'primary' | 'success' | 'neutral' | 'info' | 'warning' | 'error'>

// Static classes so Tailwind can extract them; aligned with statusColor hues.
// 'primary' is black/white in this theme, so transfer activity uses 'info' for a real hue.
const statusTextClass: Record<TorrentStatus, string> = {
  downloading: 'text-success',
  seeding: 'text-info',
  paused: 'text-dimmed',
  checking: 'text-primary',
  stalled: 'text-warning',
  error: 'text-error',
}

const columns: TableColumn<Torrent>[] = [
  {
    id: 'select',
    size: defaultColumnSizing.select,
    minSize: defaultColumnSizing.select,
    maxSize: defaultColumnSizing.select,
    enableHiding: false,
    enableResizing: false,
    enableSorting: false,
    meta: {
      class: {
        th: 'w-11 px-3',
        td: 'w-11 px-3',
      },
    },
    header: ({ table }) => h(UCheckbox, {
      modelValue: table.getIsSomeRowsSelected() ? 'indeterminate' : table.getIsAllRowsSelected(),
      'aria-label': 'Select all torrents',
      'onUpdate:modelValue': () => {
        resetSelectionAnchor()
        table.toggleAllRowsSelected(shouldSelectAllTorrents(
          table.getIsSomeRowsSelected(),
          table.getIsAllRowsSelected(),
        ))
      },
    }),
    cell: ({ row }) => h(UCheckbox, {
      modelValue: row.getIsSelected(),
      'aria-label': `Select ${row.original.name}`,
      'onUpdate:modelValue': (value: boolean | 'indeterminate') => toggleRow(row, Boolean(value)),
    }),
  },
  {
    accessorKey: 'name',
    header: resizableHeader('Torrent'),
    size: defaultColumnSizing.name,
    minSize: 180,
    maxSize: 720,
    enableHiding: false,
    meta: resizableColumnMeta,
    cell: ({ row }) => {
      const statusText = statusTextClass[row.original.status]

      return h('div', { class: 'flex w-full min-w-0 items-center gap-3' }, [
        h(UIcon, { name: statusIcon[row.original.status], class: `size-4 shrink-0 ${statusText}` }),
        h('div', { class: 'min-w-0' }, [
          h('p', { class: `truncate font-medium ${statusText}` }, row.original.name),
          h('p', { class: 'truncate text-xs text-muted' }, `${row.original.category || 'Uncategorized'} · ${formatBytes(row.original.size)}`),
        ]),
      ])
    },
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
    sortingFn: sortByStatusLabel,
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
    sortingFn: sortByEtaSeconds,
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
    sortingFn: sortByTags,
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
    enableSorting: false,
    meta: {
      class: {
        th: 'p-0',
        td: 'p-0',
      },
    },
  },
]

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

    const savedSorting = JSON.parse(localStorage.getItem(sortingStorageKey) || '[]') as unknown
    sorting.value = Array.isArray(savedSorting) ? savedSorting.filter(isSortingEntry).slice(0, 3) : []
  }
  catch {
    localStorage.removeItem(columnSizingStorageKey)
    localStorage.removeItem(columnVisibilityStorageKey)
    localStorage.removeItem(sortingStorageKey)
  }
})

onBeforeUnmount(() => {
  cancelActiveColumnResize?.()
})

watch(columnSizing, sizing => localStorage.setItem(columnSizingStorageKey, JSON.stringify(sizing)), { deep: true })
watch(columnVisibility, visibility => localStorage.setItem(columnVisibilityStorageKey, JSON.stringify(visibility)), { deep: true })
watch(sorting, state => localStorage.setItem(sortingStorageKey, JSON.stringify(state)), { deep: true })
watch(() => props.torrents, (torrents) => {
  const visibleIds = new Set(torrents.map(torrent => torrent.id))
  rowSelection.value = Object.fromEntries(Object.entries(rowSelection.value).filter(([id]) => visibleIds.has(id)))
  if (selectionAnchorId.value && !visibleIds.has(selectionAnchorId.value)) selectionAnchorId.value = undefined
})
watch(() => props.autoSelectIds, (ids) => {
  if (!ids?.length) return
  rowSelection.value = Object.fromEntries(ids.map(id => [id, true]))
  selectionAnchorId.value = ids[0]
})
</script>

<template>
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div class="flex min-w-0 items-center gap-2">
      <h1 class="text-2xl font-semibold text-highlighted">
        Torrents
      </h1>
      <UBadge v-if="selectedCount" :label="`${selectedCount} selected`" color="primary" variant="subtle" />
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <div v-if="selectedCount" class="flex items-center gap-1">
        <UButton
          label="Start"
          icon="i-lucide-play"
          color="neutral"
          variant="ghost"
          size="sm"
          aria-label="Start selected torrents"
          :disabled="activityActionsDisabled || !canStartSelected"
          :loading="actionPending"
          @click="emitSelectedActivity(false)"
        />
        <UButton
          label="Stop"
          icon="i-lucide-square"
          color="neutral"
          variant="ghost"
          size="sm"
          aria-label="Stop selected torrents"
          :disabled="activityActionsDisabled || !canStopSelected"
          :loading="actionPending"
          @click="emitSelectedActivity(true)"
        />
        <UButton
          label="Remove"
          icon="i-lucide-trash-2"
          color="error"
          variant="ghost"
          size="sm"
          aria-label="Remove selected torrents"
          :disabled="activityActionsDisabled"
          :loading="actionPending"
          @click="removeOpen = true"
        />
      </div>
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
      <slot name="actions" />
    </div>
  </div>

  <slot name="notice" />

  <UContextMenu v-if="torrents.length" :items="contextMenuItems">
    <div class="flex min-h-0 flex-1">
      <UTable
        ref="torrentTable"
        v-model:column-sizing="columnSizing"
        v-model:column-visibility="columnVisibility"
        v-model:row-selection="rowSelection"
        v-model:sorting="sorting"
        :data="torrents"
        :columns="columns"
        :column-sizing-options="{ columnResizeMode: 'onEnd' }"
        :get-row-id="torrent => torrent.id"
        :on-contextmenu="prepareRowContextMenu"
        :on-select="selectRow"
        :virtualize="{ estimateSize: 65, overscan: 8 }"
        :style="{ '--torrent-table-width': `${tableWidth}px` }"
        :ui="{ base: 'min-w-0', th: 'relative overflow-visible' }"
        class="torrent-table min-h-0 flex-1"
      />
    </div>
  </UContextMenu>

  <div v-else class="grid min-h-64 flex-1 place-items-center text-center">
    <slot name="empty" />
  </div>

  <UModal v-model:open="removeOpen" :title="removeTitle" description="The selected torrents stop being managed.">
    <template #body>
      <p class="text-sm text-muted">
        Choose <span class="text-highlighted">Remove</span> to keep downloaded content on disk, or
        <span class="text-highlighted">Remove torrent and files</span> to delete it.
      </p>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-between gap-3">
        <p class="text-xs text-muted">
          Removing cannot be undone.
        </p>
        <div class="flex gap-2">
          <UButton type="button" label="Cancel" color="neutral" variant="ghost" @click="removeOpen = false" />
          <UButton type="button" label="Remove" color="error" variant="soft" @click="confirmRemoval(false)" />
          <UButton type="button" label="Remove torrent and files" color="error" variant="solid" @click="confirmRemoval(true)" />
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

.torrent-table tbody > tr[data-selectable='true']:hover {
  background-color: color-mix(in oklab, var(--ui-bg-accented) 65%, transparent);
}

.torrent-table tbody > tr[data-selected='true'] {
  background-color: color-mix(in oklab, var(--ui-bg-accented) 78%, transparent);
}

.torrent-table tbody > tr[data-selected='true']:hover {
  background-color: color-mix(in oklab, var(--ui-bg-accented) 90%, transparent);
}

.torrent-table tbody {
  user-select: none;
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
