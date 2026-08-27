import { placeholderTorrents } from '~/data/placeholder-torrents'
import type { ActionFeedback, Torrent, TorrentAction, TorrentFilter, TorrentFilterId } from '~/types/torrent'

const clonePlaceholderTorrents = () => placeholderTorrents.map(torrent => ({
  ...torrent,
  tags: [...torrent.tags],
}))

export const useTorrentLibrary = () => {
  const torrents = ref<Torrent[]>(clonePlaceholderTorrents())
  const selectedIds = ref<string[]>(['atlas-linux'])
  const activeFilter = ref<TorrentFilterId>('all')
  const activeCategory = ref('')

  const filters = computed<TorrentFilter[]>(() => [
    { id: 'all', label: 'All torrents', icon: 'i-lucide-list-filter', count: torrents.value.length },
    { id: 'downloading', label: 'Downloading', icon: 'i-lucide-arrow-down-to-line', count: torrents.value.filter(torrent => torrent.status === 'downloading').length },
    { id: 'seeding', label: 'Seeding', icon: 'i-lucide-arrow-up-from-line', count: torrents.value.filter(torrent => torrent.status === 'seeding').length },
    { id: 'paused', label: 'Paused', icon: 'i-lucide-pause', count: torrents.value.filter(torrent => torrent.status === 'paused').length },
    { id: 'attention', label: 'Needs attention', icon: 'i-lucide-triangle-alert', count: torrents.value.filter(torrent => ['stalled', 'error'].includes(torrent.status)).length },
  ])

  const categories = computed(() => [...new Set(torrents.value.map(torrent => torrent.category))].sort())

  const visibleTorrents = computed(() => torrents.value.filter((torrent) => {
    const matchesCategory = !activeCategory.value || torrent.category === activeCategory.value
    const matchesFilter = activeFilter.value === 'all'
      || (activeFilter.value === 'attention' && ['stalled', 'error'].includes(torrent.status))
      || torrent.status === activeFilter.value

    return matchesCategory && matchesFilter
  }))

  const selectedTorrents = computed(() => torrents.value.filter(torrent => selectedIds.value.includes(torrent.id)))

  const transferTotals = computed(() => torrents.value.reduce((totals, torrent) => ({
    down: totals.down + torrent.downSpeed,
    up: totals.up + torrent.upSpeed,
  }), { down: 0, up: 0 }))

  const chooseFilter = (filter: TorrentFilterId) => {
    activeFilter.value = filter
    activeCategory.value = ''
  }

  const chooseCategory = (category: string) => {
    activeCategory.value = category
    activeFilter.value = 'all'
  }

  const toggleSelection = (id: string) => {
    selectedIds.value = selectedIds.value.includes(id)
      ? selectedIds.value.filter(selectedId => selectedId !== id)
      : [...selectedIds.value, id]
  }

  const toggleAllVisible = () => {
    const visibleIds = visibleTorrents.value.map(torrent => torrent.id)
    const allVisibleSelected = visibleIds.length > 0 && visibleIds.every(id => selectedIds.value.includes(id))

    selectedIds.value = allVisibleSelected
      ? selectedIds.value.filter(id => !visibleIds.includes(id))
      : [...new Set([...selectedIds.value, ...visibleIds])]
  }

  const execute = (action: TorrentAction, ids = selectedIds.value): ActionFeedback => {
    const targetIds = new Set(ids)
    const count = targetIds.size

    if (count === 0) {
      return { title: 'Nothing selected', description: 'Select at least one torrent first.' }
    }

    if (action === 'remove') {
      torrents.value = torrents.value.filter(torrent => !targetIds.has(torrent.id))
      selectedIds.value = selectedIds.value.filter(id => !targetIds.has(id))
      return { title: 'Torrent removed', description: `${count} placeholder torrent${count === 1 ? '' : 's'} removed.` }
    }

    torrents.value = torrents.value.map((torrent) => {
      if (!targetIds.has(torrent.id)) return torrent

      if (action === 'pause') {
        return { ...torrent, status: 'paused', downSpeed: 0, upSpeed: 0, eta: 'Paused' }
      }

      return { ...torrent, status: torrent.progress === 100 ? 'seeding' : 'downloading', eta: torrent.progress === 100 ? '∞' : '—' }
    })

    return {
      title: action === 'pause' ? 'Torrent paused' : 'Torrent resumed',
      description: `${count} placeholder torrent${count === 1 ? '' : 's'} updated.`,
    }
  }

  return {
    torrents,
    visibleTorrents,
    selectedIds,
    selectedTorrents,
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
  }
}
