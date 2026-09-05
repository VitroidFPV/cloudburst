<script setup lang="ts">
import type { TorrentContentAction, TorrentFile, TorrentFilePriorityValue, TorrentProperties, TorrentTracker } from '~/types/torrent'
import { useTorrentLibrary } from '~/composables/useTorrentLibrary'
import { formatAddedOnFull, formatBytes, formatDuration, formatEta, formatSpeed, statusColor, statusIcon, statusLabel } from '~/utils/torrent-format'

const props = defineProps<{
  torrentId: string | null
  contentActionsVisible?: boolean
  contentActionsDisabled?: boolean
  contentActionsDisabledReason?: string
}>()

const emit = defineEmits<{
  close: []
  changed: []
  'content-action': [torrentId: string, action: TorrentContentAction, fileId?: number]
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

const progressPercent = computed(() => Math.min(100, Math.max(0, torrent.value?.progress ?? 0)))
const progressLabel = computed(() => Number(progressPercent.value.toFixed(1)).toString())
const downloadedTotal = computed(() => properties.value?.downloadedTotal ?? torrent.value?.downloaded ?? 0)
const remainingBytes = computed(() => Math.max(0, (torrent.value?.size ?? 0) - downloadedTotal.value))

const properties = ref<TorrentProperties | null>(null)
const files = ref<TorrentFile[] | null>(null)
const trackers = ref<TorrentTracker[] | null>(null)
const detailsLoading = ref(false)
type DetailTab = 'overview' | 'files' | 'trackers'
const activeDetailTab = ref<DetailTab>('overview')

let pollTimer: ReturnType<typeof setTimeout> | undefined
let pollGeneration = 0

const stopPolling = () => {
  pollGeneration += 1
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
  activeDetailTab.value = 'overview'
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

const transferStats = computed(() => {
  const row = torrent.value
  if (!row) return []
  return [
    { label: 'Download', value: formatSpeed(row.downSpeed), icon: 'i-lucide-arrow-down-to-line', tone: 'text-success' },
    { label: 'Upload', value: formatSpeed(row.upSpeed), icon: 'i-lucide-arrow-up-from-line', tone: 'text-info' },
    { label: 'ETA', value: formatEta(row.etaSeconds, row.status), icon: 'i-lucide-clock-3', tone: 'text-muted' },
  ]
})

const detailStats = computed(() => {
  const row = torrent.value
  if (!row) return []
  return [
    { label: 'Total size', value: formatBytes(row.size), icon: 'i-lucide-database' },
    { label: 'Downloaded', value: formatBytes(downloadedTotal.value), icon: 'i-lucide-hard-drive-download' },
    { label: 'Uploaded', value: formatBytes(properties.value?.uploadedTotal ?? 0), icon: 'i-lucide-hard-drive-upload' },
    { label: 'Ratio', value: row.ratio.toFixed(2), icon: 'i-lucide-scale' },
    { label: 'Seeds', value: String(row.seeds), icon: 'i-lucide-sprout' },
    { label: 'Peers', value: String(row.peers), icon: 'i-lucide-users-round' },
    { label: 'Availability', value: properties.value ? properties.value.availability.toFixed(2) : '—', icon: 'i-lucide-gauge' },
    { label: 'Time active', value: properties.value ? formatDuration(properties.value.timeActive) : '—', icon: 'i-lucide-timer' },
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

const onFileContentAction = (fileIndex: number, action: TorrentContentAction) => {
  const torrentId = props.torrentId
  const file = files.value?.[fileIndex]
  if (!torrentId || !file) return
  emit('content-action', torrentId, action, file.id)
}

const trackersVisible = computed(() => (trackers.value ?? []).filter(tracker => tracker.tier >= 0))
const workingTrackerCount = computed(() => trackersVisible.value.filter(tracker => tracker.status === 2).length)
const detailTabItems = computed(() => [
  { label: 'Overview', icon: 'i-lucide-layout-dashboard', value: 'overview' },
  { label: 'Files', icon: 'i-lucide-files', value: 'files', badge: files.value === null ? undefined : files.value.length },
  { label: 'Trackers', icon: 'i-lucide-radio-tower', value: 'trackers', badge: trackers.value === null ? undefined : trackersVisible.value.length },
])

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
    <div class="flex h-(--ui-header-height) shrink-0 items-center justify-between gap-3 border-b border-default px-4 sm:px-6">
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-semibold text-highlighted" :title="torrent.name">{{ torrent.name }}</p>
      </div>
      <UButton
        type="button"
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="sm"
        aria-label="Close details"
        @click="emit('close')"
      />
    </div>

    <UTabs
      v-model="activeDetailTab"
      :items="detailTabItems"
      :content="false"
      variant="link"
      size="sm"
      class="shrink-0 border-b border-default px-4 sm:px-6"
      :ui="{ list: 'w-full', trigger: 'min-w-0 flex-1' }"
    />

    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto p-4">
      <template v-if="activeDetailTab === 'overview'">
        <section class="space-y-3">
        <div class="rounded-xl border border-default bg-(--cloudburst-detail-surface) p-4 shadow-sm">
          <div class="mb-3 flex items-start justify-between gap-3">
            <div>
              <div class="flex items-center gap-2">
                <UBadge :color="statusColor[torrent.status]" variant="subtle" size="sm" class="gap-1">
                  <UIcon :name="statusIcon[torrent.status]" class="size-3" />
                  {{ statusLabel[torrent.status] }}
                </UBadge>
                <span class="text-xs text-muted">{{ formatBytes(torrent.size) }}</span>
              </div>
              <p class="mt-2 text-sm text-highlighted">
                <span class="font-mono font-medium tabular-nums">{{ formatBytes(downloadedTotal) }}</span>
                <span class="text-muted"> of {{ formatBytes(torrent.size) }}</span>
              </p>
            </div>
            <p class="pt-0.5 font-mono text-2xl font-semibold leading-none tabular-nums text-highlighted">
              {{ progressLabel }}<span class="text-sm text-muted">%</span>
            </p>
          </div>
          <UProgress
            :model-value="progressPercent"
            :max="100"
            :color="statusColor[torrent.status]"
            size="md"
            aria-label="Torrent progress"
          />
          <div class="mt-2.5 flex items-center justify-between gap-4 text-xs text-muted">
            <span>{{ formatBytes(remainingBytes) }} remaining</span>
            <span v-if="torrent.status === 'downloading'">{{ formatEta(torrent.etaSeconds, torrent.status) }} left</span>
            <span v-else>{{ statusLabel[torrent.status] }}</span>
          </div>
        </div>

        <dl class="grid grid-cols-3 gap-2">
          <div
            v-for="stat in transferStats"
            :key="stat.label"
            class="min-w-0 rounded-lg border border-default bg-(--cloudburst-detail-surface) px-3 py-2.5"
          >
            <dt class="flex items-center gap-1.5 text-xs text-muted">
              <UIcon :name="stat.icon" class="size-3.5" :class="stat.tone" />
              {{ stat.label }}
            </dt>
            <dd class="mt-1 truncate font-mono text-sm font-medium tabular-nums text-highlighted" :title="stat.value">{{ stat.value }}</dd>
          </div>
        </dl>
        </section>

        <section class="space-y-3">
        <div class="flex items-center gap-2">
          <UIcon name="i-lucide-activity" class="size-4 text-muted" />
          <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Activity</h3>
        </div>
        <dl class="grid grid-cols-2 overflow-hidden rounded-lg border border-default bg-(--cloudburst-detail-surface)">
          <div
            v-for="stat in detailStats"
            :key="stat.label"
            class="flex min-w-0 items-center gap-2.5 border-b border-default px-3 py-2.5 odd:border-r [&:nth-last-child(-n+2)]:border-b-0"
          >
            <UIcon :name="stat.icon" class="size-4 shrink-0 text-muted" />
            <div class="min-w-0">
              <dt class="text-[11px] text-muted">{{ stat.label }}</dt>
              <dd class="truncate font-mono text-sm tabular-nums text-highlighted">{{ stat.value }}</dd>
            </div>
          </div>
        </dl>
        </section>

        <section class="space-y-3">
        <div class="flex items-center gap-2">
          <UIcon name="i-lucide-info" class="size-4 text-muted" />
          <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Details</h3>
        </div>
        <dl class="overflow-hidden rounded-lg border border-default bg-(--cloudburst-detail-surface) text-sm">
          <div class="flex items-center gap-3 border-b border-default px-3 py-2.5">
            <UIcon name="i-lucide-calendar-plus" class="size-4 shrink-0 text-muted" />
            <dt class="w-20 shrink-0 text-muted">Added</dt>
            <dd class="min-w-0 flex-1 truncate text-right text-highlighted" :title="formatAddedOnFull(torrent.addedOn)">{{ formatAddedOnFull(torrent.addedOn) }}</dd>
          </div>
          <div v-if="properties?.completedOn" class="flex items-center gap-3 border-b border-default px-3 py-2.5">
            <UIcon name="i-lucide-circle-check" class="size-4 shrink-0 text-success" />
            <dt class="w-20 shrink-0 text-muted">Completed</dt>
            <dd class="min-w-0 flex-1 truncate text-right text-highlighted">{{ formatAddedOnFull(properties.completedOn) }}</dd>
          </div>
          <div class="flex items-center gap-3 px-3 py-2.5">
            <UIcon name="i-lucide-folder-down" class="size-4 shrink-0 text-muted" />
            <dt class="w-20 shrink-0 text-muted">Location</dt>
            <dd class="min-w-0 flex-1 truncate text-right font-mono text-xs text-highlighted" :title="torrent.savePath">{{ torrent.savePath }}</dd>
          </div>
        </dl>

        <div class="grid gap-3 sm:grid-cols-2">
          <UFormField label="Category">
            <UInputMenu
              v-model="categoryModel"
              :items="categoryOptions"
              create-item
              size="sm"
              placeholder="Uncategorized"
              aria-label="Category"
              class="w-full"
              :ui="{ base: 'min-h-8' }"
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
              class="w-full"
              :ui="{ base: 'min-h-8' }"
              @update:model-value="onTagsChange"
            />
          </UFormField>
        </div>
        </section>
      </template>

      <section v-else-if="activeDetailTab === 'files'" class="space-y-2">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-files" class="size-4 text-muted" />
            <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Files</h3>
          </div>
          <UBadge v-if="files" :label="String(files.length)" color="neutral" variant="subtle" size="sm" />
        </div>
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
          :content-actions="contentActionsVisible"
          :content-actions-disabled="contentActionsDisabled"
          :content-actions-disabled-reason="contentActionsDisabledReason"
          @change="onTreeChange"
          @content-action="onFileContentAction"
        />
        <p v-else class="py-4 text-center text-sm text-muted">
          This torrent has no files to pick from.
        </p>
      </section>

      <section v-else class="space-y-2">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-radio-tower" class="size-4 text-muted" />
            <h3 class="text-xs font-medium uppercase tracking-wide text-muted">Trackers</h3>
          </div>
          <span v-if="trackers !== null" class="text-xs text-muted">{{ workingTrackerCount }} of {{ trackersVisible.length }} working</span>
        </div>
        <p v-if="trackers === null" class="py-4 text-center text-sm text-muted">
          The tracker list is unavailable right now.
        </p>
        <p v-else-if="!trackersVisible.length" class="py-4 text-center text-sm text-muted">
          No trackers to show.
        </p>
        <ul v-else class="space-y-1.5">
          <li
            v-for="tracker in trackersVisible"
            :key="`${tracker.tier}:${tracker.url}`"
            class="rounded-lg border border-default bg-(--cloudburst-detail-surface) px-3 py-2"
          >
            <div class="flex items-center gap-2.5">
              <span class="size-2 shrink-0 rounded-full" :class="trackerStatusMeta(tracker.status).tone" />
              <span class="min-w-0 flex-1 truncate font-mono text-xs text-highlighted">{{ trackerHost(tracker.url) }}</span>
              <span class="shrink-0 text-[11px] text-muted">{{ trackerStatusMeta(tracker.status).label }}</span>
            </div>
            <div class="mt-1.5 flex items-center justify-between gap-3 pl-4.5 text-[11px] text-muted">
              <span class="truncate">{{ tracker.message || (tracker.status === 2 ? 'Announce healthy' : 'No tracker message') }}</span>
              <span class="shrink-0 font-mono tabular-nums">{{ tracker.seeds }} seeds · {{ tracker.leeches }} leeches</span>
            </div>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>
