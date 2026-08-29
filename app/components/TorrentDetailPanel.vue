<script setup lang="ts">
import type { TorrentFile, TorrentFilePriorityValue, TorrentProperties, TorrentTracker } from '~/types/torrent'
import { useTorrentLibrary } from '~/composables/useTorrentLibrary'
import { formatAddedOnFull, formatBytes, formatDuration, formatEta, formatSpeed, statusColor, statusLabel } from '~/utils/torrent-format'

const props = defineProps<{
  torrentId: string | null
}>()

const emit = defineEmits<{
  close: []
  changed: []
}>()

const DETAIL_POLL_INTERVAL_MS = 1_000

const {
  torrents,
  fetchTorrentProperties,
  fetchTorrentFiles,
  fetchTorrentTrackers,
  setTorrentFilePriorities,
  setTorrentCategory,
  addTorrentTags,
  removeTorrentTags,
  fetchCategories,
  fetchTags,
  categories,
} = useTorrentLibrary()

const torrent = computed(() => torrents.value.find(candidate => candidate.id === props.torrentId) ?? null)

const properties = ref<TorrentProperties | null>(null)
const files = ref<TorrentFile[] | null>(null)
const trackers = ref<TorrentTracker[] | null>(null)
const detailsLoading = ref(false)

let pollTimer: ReturnType<typeof setTimeout> | undefined
let pollGeneration = 0

const stopPolling = () => {
  if (pollTimer) {
    clearTimeout(pollTimer)
    pollTimer = undefined
  }
}

const schedulePoll = (torrentId: string) => {
  pollTimer = setTimeout(() => void poll(torrentId), DETAIL_POLL_INTERVAL_MS)
}

const poll = async (torrentId: string) => {
  const generation = ++pollGeneration
  const [nextProperties, nextFiles, nextTrackers] = await Promise.all([
    fetchTorrentProperties(torrentId),
    fetchTorrentFiles(torrentId),
    fetchTorrentTrackers(torrentId),
  ])

  if (generation !== pollGeneration || props.torrentId !== torrentId) return

  if (nextProperties) properties.value = nextProperties
  if (nextFiles) files.value = nextFiles
  if (nextTrackers) trackers.value = nextTrackers
  detailsLoading.value = false

  schedulePoll(torrentId)
}

const resetDetails = () => {
  stopPolling()
  properties.value = null
  files.value = null
  trackers.value = null
  detailsLoading.value = props.torrentId !== null
}

const categoryModel = ref<string>()
const categoryOptions = ref<string[]>([])
const lastSyncedCategory = ref<string | undefined>()

const loadCategoryOptions = async () => {
  const fetched = await fetchCategories()
  categoryOptions.value = fetched ?? [...categories.value]
}

const tagModel = ref<string[]>([])
const tagOptions = ref<string[]>([])
const lastSyncedTags = ref<string[]>([])

const loadTagOptions = async () => {
  const fetched = await fetchTags()
  tagOptions.value = fetched ?? []
}

watch(() => props.torrentId, () => {
  resetDetails()
  if (props.torrentId) {
    void poll(props.torrentId)
    void loadCategoryOptions()
    void loadTagOptions()
  }
}, { immediate: true })

onBeforeUnmount(stopPolling)

// The panel is bound to the torrent it was opened with; if that torrent
// leaves the library, there is nothing left to show.
watch(torrent, (value) => {
  if (!value && props.torrentId) emit('close')
})

const stats = computed(() => {
  const row = torrent.value
  if (!row) return []
  return [
    { label: 'Down', value: formatSpeed(row.downSpeed) },
    { label: 'Up', value: formatSpeed(row.upSpeed) },
    { label: 'ETA', value: formatEta(row.etaSeconds, row.status) },
    { label: 'Ratio', value: row.ratio.toFixed(2) },
    { label: 'Downloaded', value: formatBytes(properties.value?.downloadedTotal ?? row.downloaded) },
    { label: 'Uploaded', value: formatBytes(properties.value?.uploadedTotal ?? 0) },
    { label: 'Seeds', value: String(row.seeds) },
    { label: 'Peers', value: String(row.peers) },
    { label: 'Availability', value: properties.value ? properties.value.availability.toFixed(2) : '—' },
    { label: 'Time active', value: properties.value ? formatDuration(properties.value.timeActive) : '—' },
  ]
})

const treeFiles = computed(() => (files.value ?? []).map(file => ({
  path: file.path,
  length: file.size,
  progress: file.progress,
})))

const instancePriorities = computed(() => (files.value ?? []).map(file => file.priority))

const knownPriorities = ref<number[]>([])
watch(instancePriorities, (value) => {
  knownPriorities.value = [...value]
}, { immediate: true })

const onTreeChange = (payload: { priorities: number[], selectedSize: number, allSelected: boolean }) => {
  if (!props.torrentId) return
  const updates = payload.priorities
    .map((priority, id) => ({ id, priority }))
    .filter(update => knownPriorities.value[update.id] !== undefined && knownPriorities.value[update.id] !== update.priority)
    .map(update => ({ id: update.id, priority: update.priority as TorrentFilePriorityValue }))
  knownPriorities.value = [...payload.priorities]
  if (updates.length) void setTorrentFilePriorities(props.torrentId, updates)
}

const trackersVisible = computed(() => (trackers.value ?? []).filter(tracker => tracker.tier >= 0))

const trackerStatusMeta = (status: number): { label: string, tone: string } => {
  if (status === 0) return { label: 'Disabled', tone: 'bg-neutral-500' }
  if (status === 1) return { label: 'Not contacted', tone: 'bg-muted' }
  if (status === 2) return { label: 'Working', tone: 'bg-success' }
  if (status === 3) return { label: 'Updating', tone: 'bg-info' }
  if (status === 4) return { label: 'Not working', tone: 'bg-error' }
  return { label: 'Unreachable', tone: 'bg-warning' }
}

const trackerHost = (url: string) => {
  try {
    return new URL(url).host
  }
  catch {
    return url
  }
}

watch(torrent, (value) => {
  if (!value) return
  const next = value.category || undefined
  if (next !== categoryModel.value) {
    categoryModel.value = next
  }
  lastSyncedCategory.value = next
}, { immediate: true })

const onCategoryChange = async (value: string | null | undefined) => {
  const next = value ?? undefined
  if (!props.torrentId || next === lastSyncedCategory.value) return
  if (await setTorrentCategory([props.torrentId], next ?? '')) emit('changed')
}

watch(torrent, (value) => {
  if (!value) return
  const next = [...value.tags]
  if (JSON.stringify(next) !== JSON.stringify(tagModel.value)) {
    tagModel.value = next
  }
  lastSyncedTags.value = next
}, { immediate: true })

const onTagsChange = async (value: string[]) => {
  if (!torrent.value) return
  const added = value.filter(tag => !lastSyncedTags.value.includes(tag))
  const removed = lastSyncedTags.value.filter(tag => !value.includes(tag))
  if (!added.length && !removed.length) return

  const torrentId = torrent.value.id
  if (added.length) void addTorrentTags([torrentId], added)
  if (removed.length) void removeTorrentTags([torrentId], removed)
  lastSyncedTags.value = [...value]
  tagModel.value = [...value]
  emit('changed')
}
</script>

<template>
  <div v-if="torrent" class="flex h-full min-h-0 flex-col">
    <div class="flex items-start justify-between gap-3 border-b border-default p-4">
      <div class="min-w-0">
        <p class="truncate text-sm font-medium text-highlighted">{{ torrent.name }}</p>
        <div class="mt-1.5 flex items-center gap-2">
          <UBadge :color="statusColor[torrent.status]" variant="subtle" size="sm">
            {{ statusLabel[torrent.status] }}
          </UBadge>
          <span class="font-mono text-xs text-muted">{{ Math.round(torrent.progress * 100) }}%</span>
        </div>
      </div>
      <UButton
        type="button"
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="xs"
        aria-label="Close details"
        @click="emit('close')"
      />
    </div>

    <div class="min-h-0 flex-1 space-y-6 overflow-y-auto p-4">
      <section class="space-y-3">
        <UProgress :model-value="Math.round(torrent.progress * 100)" size="sm" />
        <dl class="grid grid-cols-2 gap-x-4 gap-y-2.5 sm:grid-cols-3">
          <div v-for="stat in stats" :key="stat.label">
            <dt class="text-xs text-muted">{{ stat.label }}</dt>
            <dd class="truncate font-mono text-sm text-highlighted">{{ stat.value }}</dd>
          </div>
        </dl>
      </section>

      <section class="space-y-3">
        <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Facts</h3>
        <dl class="space-y-2.5 text-sm">
          <div class="flex justify-between gap-4">
            <dt class="shrink-0 text-muted">Added</dt>
            <dd class="truncate text-highlighted" :title="formatAddedOnFull(torrent.addedOn)">{{ formatAddedOnFull(torrent.addedOn) }}</dd>
          </div>
          <div v-if="properties?.completedOn" class="flex justify-between gap-4">
            <dt class="shrink-0 text-muted">Completed</dt>
            <dd class="truncate text-highlighted">{{ formatAddedOnFull(properties.completedOn) }}</dd>
          </div>
          <div class="flex justify-between gap-4">
            <dt class="shrink-0 text-muted">Save location</dt>
            <dd class="truncate font-mono text-xs text-highlighted" :title="torrent.savePath">{{ torrent.savePath }}</dd>
          </div>
        </dl>

        <UFormField label="Category">
          <UInputMenu
            v-model="categoryModel"
            :items="categoryOptions"
            create-item
            size="sm"
            placeholder="Uncategorized"
            aria-label="Category"
            @update:model-value="onCategoryChange"
          />
        </UFormField>

        <UFormField label="Tags">
          <UInputMenu
            v-model="tagModel"
            :items="tagOptions"
            multiple
            create-item
            size="sm"
            placeholder="No tags"
            aria-label="Tags"
            @update:model-value="onTagsChange"
          />
        </UFormField>
      </section>

      <section class="space-y-2">
        <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Files</h3>
        <div v-if="detailsLoading && !files" class="flex items-center justify-center gap-2 py-10 text-sm text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
          Fetching details…
        </div>
        <p v-else-if="!files" class="py-4 text-center text-sm text-muted">
          The file list is unavailable right now.
        </p>
        <TorrentFileTree
          v-else-if="files.length"
          :files="treeFiles"
          :priorities="instancePriorities"
          :reset-key="torrent.id"
          @change="onTreeChange"
        />
        <p v-else class="py-4 text-center text-sm text-muted">
          This torrent has no files to pick from.
        </p>
      </section>

      <section class="space-y-2">
        <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Trackers</h3>
        <p v-if="trackers === null" class="py-4 text-center text-sm text-muted">
          The tracker list is unavailable right now.
        </p>
        <p v-else-if="!trackersVisible.length" class="py-4 text-center text-sm text-muted">
          No trackers to show.
        </p>
        <ul v-else class="space-y-1.5">
          <li
            v-for="tracker in trackersVisible"
            :key="tracker.url"
            class="rounded-md bg-elevated/50 px-2.5 py-1.5"
          >
            <div class="flex items-center gap-2">
              <span class="size-2 shrink-0 rounded-full" :class="trackerStatusMeta(tracker.status).tone" />
              <span class="min-w-0 flex-1 truncate font-mono text-xs text-highlighted">{{ trackerHost(tracker.url) }}</span>
              <span class="shrink-0 text-xs text-muted">{{ trackerStatusMeta(tracker.status).label }}</span>
              <span class="shrink-0 font-mono text-xs text-muted">{{ tracker.seeds }}↑ {{ tracker.leeches }}↓</span>
            </div>
            <p v-if="tracker.message" class="mt-1 truncate text-xs text-warning" :title="tracker.message">{{ tracker.message }}</p>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>
