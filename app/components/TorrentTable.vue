<script setup lang="ts">
import type { ContextMenuItem, TableColumn, TableRow } from '@nuxt/ui'
import type { Torrent, TorrentContentAction, TorrentStatus } from '~/types/torrent'
import { formatAddedOn, formatAddedOnFull, formatBytes, formatEta, formatSpeed, statusColor, statusIcon, statusLabel } from '~/utils/torrent-format'
import { resolveTorrentSelection, shouldSelectAllTorrents } from '~/utils/torrent-selection'

const props = defineProps<{
  torrents: Torrent[]
  actionsDisabled?: boolean
  actionPending?: boolean
  autoSelectIds?: string[]
  categories?: string[]
  tags?: string[]
  openTorrentId?: string | null
  contentActionsVisible?: boolean
  contentActionsDisabled?: boolean
  contentActionsDisabledReason?: string
}>()

const emit = defineEmits<{
  'set-paused': [torrentIds: string[], paused: boolean]
  'remove-torrents': [torrentIds: string[], deleteFiles: boolean]
  'set-category': [torrentIds: string[], category: string]
  'add-tags': [torrentIds: string[], tags: string[]]
  'remove-tags': [torrentIds: string[], tags: string[]]
  'toggle-details': [torrentId: string]
  'content-action': [torrentId: string, action: TorrentContentAction]
}>()

// UTable has no double-click hook, so a second click on the same row within
// the threshold counts as toggling the torrent's details.
const DOUBLE_CLICK_MS = 400
let lastClick: { id: string, at: number, shiftKey: boolean } | undefined

const UBadge = resolveComponent('UBadge')
const UIcon = resolveComponent('UIcon')
const UProgress = resolveComponent('UProgress')
const UButton = resolveComponent('UButton')
const UCheckbox = resolveComponent('UCheckbox')

const tableRoot = useTemplateRef<HTMLElement>('tableRoot')
const searchQuery = ref('')
const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLocaleLowerCase())
const filteredTorrents = computed(() => {
  const query = normalizedSearchQuery.value
  if (!query) return props.torrents

  return props.torrents.filter(torrent => [
    torrent.name,
    torrent.category,
    torrent.tags.join(' '),
    statusLabel[torrent.status],
  ].some(value => value.toLocaleLowerCase().includes(query)))
})

const focusSearch = () => {
  tableRoot.value?.querySelector<HTMLInputElement>('#torrent-search')?.focus()
}

defineShortcuts({
  '/': focusSearch,
  meta_f: focusSearch,
})

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
const hiddenSelectedCount = computed(() => {
  const visibleIds = new Set(filteredTorrents.value.map(torrent => torrent.id))
  return selectedTorrents.value.filter(torrent => !visibleIds.has(torrent.id)).length
})
const selectionLabel = computed(() => `${selectedCount.value} selected${hiddenSelectedCount.value ? ` · ${hiddenSelectedCount.value} hidden` : ''}`)
const canStartSelected = computed(() => selectedTorrents.value.some(torrent => torrent.status === 'paused'))
const canStopSelected = computed(() => selectedTorrents.value.some(torrent => torrent.status !== 'paused'))
const activityActionsDisabled = computed(() => props.actionsDisabled || props.actionPending)
const detailsOpenForSelection = computed(() => selectedCount.value === 1
  && selectedTorrents.value[0]!.id === props.openTorrentId)

const emitSelectedActivity = (paused: boolean) => {
  if (!selectedCount.value || activityActionsDisabled.value) return
  emit('set-paused', selectedTorrents.value.map(torrent => torrent.id), paused)
}

const emitSelectedContentAction = (action: TorrentContentAction) => {
  const torrent = selectedTorrents.value[0]
  if (!torrent || selectedCount.value !== 1 || props.contentActionsDisabled) return
  emit('content-action', torrent.id, action)
}

const removeOpen = ref(false)
const removeFiles = ref(false)
const removeTitle = computed(() => `Remove ${selectedCount.value === 1 ? 'torrent' : `${selectedCount.value} torrents`}`)

watch(removeOpen, (open) => {
  if (open) removeFiles.value = false
})

const confirmRemoval = (deleteFiles: boolean) => {
  if (!selectedCount.value || activityActionsDisabled.value) return
  removeOpen.value = false
  emit('remove-torrents', selectedTorrents.value.map(torrent => torrent.id), deleteFiles)
}

const contextMenuItems = computed<ContextMenuItem[][]>(() => {
  const primaryItems: ContextMenuItem[] = [
    {
      label: detailsOpenForSelection.value ? 'Close details' : 'Details',
      icon: detailsOpenForSelection.value ? 'i-lucide-panel-right-close' : 'i-lucide-panel-right-open',
      disabled: selectedCount.value !== 1,
      onSelect: () => emit('toggle-details', selectedTorrents.value[0]!.id),
    },
  ]

  if (props.contentActionsVisible) {
    primaryItems.push(
      {
        label: 'Open',
        icon: 'i-lucide-external-link',
        disabled: selectedCount.value !== 1 || props.contentActionsDisabled,
        onSelect: () => emitSelectedContentAction('open'),
      },
      {
        label: 'Show in folder',
        icon: 'i-lucide-folder-open',
        disabled: selectedCount.value !== 1 || props.contentActionsDisabled,
        onSelect: () => emitSelectedContentAction('reveal'),
      },
    )
  }

  primaryItems.push(
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
  )
  const groups: ContextMenuItem[][] = [primaryItems]

  const selectedIds = selectedTorrents.value.map(torrent => torrent.id)
  const organizeItems: ContextMenuItem[] = []

  if (props.categories?.length) {
    organizeItems.push({
      label: 'Set category',
      icon: 'i-lucide-folder-input',
      children: [
        ...props.categories.map(category => ({
          label: category,
          icon: selectionCategory.value === category ? 'i-lucide-check' : undefined,
          onSelect: () => emit('set-category', selectedIds, category),
        })),
        {
          label: 'No category',
          icon: 'i-lucide-folder-minus',
          disabled: !selectionCategory.value,
          onSelect: () => emit('set-category', selectedIds, ''),
        },
      ],
    })
  }

  if (props.tags?.length) {
    organizeItems.push({
      label: 'Tags',
      icon: 'i-lucide-tags',
      children: props.tags.map((tag) => {
        const onEverySelection = selectedTorrents.value.every(torrent => torrent.tags.includes(tag))
        return {
          label: tag,
          icon: onEverySelection ? 'i-lucide-check' : undefined,
          onSelect: () => (onEverySelection
            ? emit('remove-tags', selectedIds, [tag])
            : emit('add-tags', selectedIds, [tag])),
        }
      }),
    })
  }

  if (organizeItems.length) groups.push(organizeItems)

  groups.push([
    {
      label: 'Remove…',
      icon: 'i-lucide-trash-2',
      disabled: activityActionsDisabled.value,
      onSelect: () => {
        removeOpen.value = true
      },
    },
  ])

  return groups
})

const selectionCategory = computed(() => {
  const first = selectedTorrents.value[0]
  if (!first) return undefined
  return selectedTorrents.value.every(torrent => torrent.category === first.category)
    ? first.category
    : undefined
})

const selectRow = (event: Event, row: TableRow<Torrent>) => {
  const mouseEvent = event as MouseEvent
  const additive = mouseEvent.ctrlKey || mouseEvent.metaKey

  const now = Date.now()
  const isDoubleClick = !additive
    && lastClick?.id === row.id
    && lastClick.shiftKey === mouseEvent.shiftKey
    && now - lastClick.at <= DOUBLE_CLICK_MS
  lastClick = { id: row.id, at: now, shiftKey: mouseEvent.shiftKey }
  if (isDoubleClick) {
    lastClick = undefined
    if (!mouseEvent.shiftKey) {
      emit('toggle-details', row.original.id)
      return
    }
    if (props.contentActionsVisible && !props.contentActionsDisabled) {
      emit('content-action', row.original.id, 'open')
      return
    }
  }

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

const openFocusedRow = (event: KeyboardEvent) => {
  const target = event.target as HTMLElement | null
  const rowElement = target?.closest<HTMLTableRowElement>('tbody tr')
  if (!rowElement || target !== rowElement) return

  const rowIndex = Number(rowElement.style.getPropertyValue('--torrent-row-index'))
  const row = torrentTable.value?.tableApi.getRowModel().rows.find(candidate => candidate.index === rowIndex)
  if (!row) return

  event.preventDefault()
  emit('toggle-details', row.original.id)
}

const clearSelection = () => {
  rowSelection.value = {}
  selectionAnchorId.value = undefined
}

// Escape closes the open details first; with details closed it clears the
// selection. Open dialogs and menus keep their own Escape handling.
const onDocumentKeydown = (event: KeyboardEvent) => {
  if (event.key !== 'Escape' || event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return
  if (document.querySelector('[role="dialog"], [role="menu"], [role="listbox"]')) return

  if (props.openTorrentId) {
    emit('toggle-details', props.openTorrentId)
    return
  }
  if (selectedCount.value) clearSelection()
}

const tableMeta = {
  class: { tr: 'group' },
  style: {
    tr: (row: TableRow<Torrent>) => ({ '--torrent-row-index': String(row.index) }),
  },
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
        'after:absolute after:inset-y-2 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-(--ui-border-accented) after:transition-colors',
        'hover:after:bg-primary focus-visible:after:w-0.5 focus-visible:after:bg-primary',
        'data-[resizing=true]:after:w-0.5 data-[resizing=true]:after:bg-primary',
      ],
      onPointerdown: (event: PointerEvent) => startColumnResize(event, header.column),
      onDblclick: () => header.column.resetSize(),
      onKeydown: (event: KeyboardEvent) => resizeColumnWithKeyboard(event, header.column),
    }),
  ])
}

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

const torrentNameTextClass: Record<TorrentStatus, string> = {
  ...statusTextClass,
  paused: 'text-(--cloudburst-paused-name)',
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
      const nameText = torrentNameTextClass[row.original.status]
      const detailsOpen = props.openTorrentId === row.original.id

      return h('div', { class: 'flex w-full min-w-0 items-center gap-3' }, [
        h(UIcon, { name: statusIcon[row.original.status], class: `size-4 shrink-0 ${statusText}` }),
        h('div', { class: 'min-w-0 flex-1' }, [
          h('p', { class: `truncate font-medium ${nameText}` }, row.original.name),
          h('p', { class: 'truncate text-xs text-muted' }, `${row.original.category || 'Uncategorized'} · ${formatBytes(row.original.size)}`),
        ]),
        h(UButton, {
          type: 'button',
          icon: 'i-lucide-chevron-right',
          color: 'neutral',
          variant: 'ghost',
          size: 'xs',
          class: [
            'shrink-0 transition-all',
            detailsOpen
              ? 'rotate-180 opacity-100'
              : 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100',
          ],
          'aria-label': `${detailsOpen ? 'Close' : 'Open'} details for ${row.original.name}`,
          'aria-expanded': detailsOpen,
          title: `${detailsOpen ? 'Close' : 'Open'} details for ${row.original.name}`,
          onClick: (event: MouseEvent) => {
            event.stopPropagation()
            emit('toggle-details', row.original.id)
          },
        }),
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
        th: 'layout-spacer p-0',
        td: 'layout-spacer p-0',
      },
    },
  },
]

onMounted(() => {
  document.addEventListener('keydown', onDocumentKeydown)
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
  document.removeEventListener('keydown', onDocumentKeydown)
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
  <div ref="tableRoot" class="flex min-h-0 flex-1 flex-col">
    <div class="flex h-(--ui-header-height) shrink-0 items-center justify-between gap-3 border-b border-default px-4 sm:px-6">
      <div class="flex min-w-0 flex-1 items-center gap-2">
        <UInput
          id="torrent-search"
          v-model="searchQuery"
          icon="i-lucide-search"
          size="sm"
          placeholder="Search torrents…"
          aria-label="Search torrents"
          class="min-w-32 max-w-lg flex-1"
          :disabled="!torrents.length"
        >
          <template v-if="searchQuery" #trailing>
            <UButton
              type="button"
              icon="i-lucide-x"
              color="neutral"
              variant="link"
              size="xs"
              aria-label="Clear torrent search"
              @click="searchQuery = ''"
            />
          </template>
        </UInput>
        <template v-if="selectedCount">
          <UBadge :label="selectionLabel" color="primary" variant="subtle" aria-live="polite" />
          <UButton icon="i-lucide-x" color="neutral" variant="ghost" size="xs" aria-label="Clear selection" title="Clear selection" @click="clearSelection" />
        </template>
        <span v-else-if="normalizedSearchQuery" class="shrink-0 text-xs text-muted">
          {{ filteredTorrents.length }} of {{ torrents.length }}
        </span>
      </div>

      <div class="flex shrink-0 items-center gap-1">
        <div v-if="selectedCount" class="flex items-center gap-1 border-r border-default pr-1">
          <UButton
            icon="i-lucide-play"
            color="neutral"
            variant="ghost"
            size="sm"
            aria-label="Start selected torrents"
            title="Start selected torrents"
            :disabled="activityActionsDisabled || !canStartSelected"
            :loading="actionPending"
            @click="emitSelectedActivity(false)"
          />
          <UButton
            icon="i-lucide-square"
            color="neutral"
            variant="ghost"
            size="sm"
            aria-label="Stop selected torrents"
            title="Stop selected torrents"
            :disabled="activityActionsDisabled || !canStopSelected"
            :loading="actionPending"
            @click="emitSelectedActivity(true)"
          />
          <UButton
            v-if="contentActionsVisible"
            icon="i-lucide-external-link"
            color="neutral"
            variant="ghost"
            size="sm"
            aria-label="Open selected torrent content"
            :title="selectedCount === 1 ? (contentActionsDisabledReason || 'Open content · Shift + double-click') : 'Select one torrent to open its content.'"
            :disabled="selectedCount !== 1 || contentActionsDisabled"
            @click="emitSelectedContentAction('open')"
          />
          <UButton
            v-if="contentActionsVisible"
            icon="i-lucide-folder-open"
            color="neutral"
            variant="ghost"
            size="sm"
            aria-label="Show selected torrent content in folder"
            :title="selectedCount === 1 ? (contentActionsDisabledReason || 'Show content in folder') : 'Select one torrent to show its content in a folder.'"
            :disabled="selectedCount !== 1 || contentActionsDisabled"
            @click="emitSelectedContentAction('reveal')"
          />
          <UButton
            icon="i-lucide-trash-2"
            color="neutral"
            variant="ghost"
            size="sm"
            aria-label="Remove selected torrents"
            title="Remove selected torrents"
            :disabled="activityActionsDisabled"
            :loading="actionPending"
            @click="removeOpen = true"
          />
        </div>
        <UDropdownMenu :items="columnVisibilityItems" :content="{ align: 'end' }">
          <UButton
            icon="i-lucide-columns-2"
            color="neutral"
            variant="ghost"
            size="sm"
            aria-label="Choose visible columns"
            title="Choose visible columns"
          />
        </UDropdownMenu>
        <span class="mx-1 h-5 border-l border-default" />
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
          :data="filteredTorrents"
          :columns="columns"
          :column-sizing-options="{ columnResizeMode: 'onEnd' }"
          :get-row-id="torrent => torrent.id"
          :on-contextmenu="prepareRowContextMenu"
          :on-select="selectRow"
          :meta="tableMeta"
          :virtualize="filteredTorrents.length > 50 ? { estimateSize: 65, overscan: 8 } : false"
          :style="{ '--torrent-table-width': `${tableWidth}px` }"
          :ui="{ base: 'min-w-0', th: 'relative overflow-visible' }"
          class="torrent-table min-h-0 flex-1"
          @keydown.enter="openFocusedRow"
        >
          <template #empty>
            <div class="grid min-h-56 place-items-center px-6 text-center">
              <div>
                <UIcon name="i-lucide-search-x" class="mx-auto size-8 text-muted" />
                <p class="mt-3 font-medium text-highlighted">No matching torrents</p>
                <p class="mt-1 text-sm text-muted">Try another name, category, tag, or status.</p>
                <UButton class="mt-3" label="Clear search" color="neutral" variant="soft" size="sm" @click="searchQuery = ''" />
              </div>
            </div>
          </template>
        </UTable>
      </div>
    </UContextMenu>

    <div v-else class="grid min-h-64 flex-1 place-items-center px-6 text-center">
      <slot name="empty" />
    </div>

    <UModal v-model:open="removeOpen" :title="removeTitle" description="The selected torrents stop being managed.">
      <template #body>
        <ul class="mb-4 max-h-48 space-y-1 overflow-y-auto text-sm text-highlighted" aria-label="Torrents to remove">
          <li v-for="torrent in selectedTorrents" :key="torrent.id" class="break-words">{{ torrent.name }}</li>
        </ul>
        <p v-if="hiddenSelectedCount" class="mb-4 text-sm text-muted">{{ hiddenSelectedCount }} selected {{ hiddenSelectedCount === 1 ? 'torrent is' : 'torrents are' }} hidden by your search.</p>
        <div class="flex items-start justify-between gap-3">
          <div>
            <p class="text-sm font-medium text-highlighted">Also remove downloaded files</p>
            <p class="mt-0.5 text-xs text-muted">Deletes the downloaded content from disk.</p>
          </div>
          <USwitch v-model="removeFiles" aria-label="Also remove downloaded files" />
        </div>
      </template>

      <template #footer>
        <div class="flex w-full items-center justify-between gap-3">
          <p class="text-xs text-muted">
            {{ removeFiles ? 'Downloaded files will be permanently deleted.' : 'Downloaded files will be kept.' }}
          </p>
          <div class="flex gap-2">
            <UButton type="button" label="Cancel" color="neutral" variant="ghost" @click="removeOpen = false" />
            <UButton type="button" :label="removeFiles ? 'Remove torrent and files' : 'Remove torrent'" color="error" variant="solid" :disabled="activityActionsDisabled || !selectedCount" @click="confirmRemoval(removeFiles)" />
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<style>
.torrent-table table {
  width: 100%;
  min-width: var(--torrent-table-width);
  table-layout: fixed;
}

.torrent-table .layout-spacer {
  width: 100%;
}

.torrent-table tbody > tr[data-selectable='true']:hover > td {
  background-color: color-mix(in oklab, var(--ui-bg-accented) 65%, transparent);
}

.torrent-table tbody > tr[data-selected='true'] > td {
  background-color: color-mix(in oklab, var(--ui-primary) 12%, var(--ui-bg));
}

.torrent-table tbody > tr[data-selected='true'] > td:first-child {
  box-shadow: inset 2px 0 var(--ui-primary);
}

.torrent-table tbody > tr[data-selected='true']:hover > td {
  background-color: color-mix(in oklab, var(--ui-primary) 16%, var(--ui-bg));
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
