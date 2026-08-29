<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import type { NavigationMenuItem } from '@nuxt/ui'
import type { AddTorrentsInput, AuthenticationMode, ConnectionInput, MagnetHandlerStatus } from '~/types/torrent'
import { useTorrentLibrary } from '~/composables/useTorrentLibrary'
import { isLoopbackEndpoint } from '~/utils/connection'
import { formatSpeed } from '~/utils/torrent-format'

interface AddTorrentModalApi {
  openWith: (options?: { urls?: string[], files?: File[] }) => void
  close: () => void
}

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
  activityUpdating,
  torrentActionError,
  defaultSavePath,
  connect,
  restoreSavedConnection,
  retry,
  startAutoRefresh,
  setTorrentsPaused,
  removeTorrents,
  addTorrents,
  loadDefaultSavePath,
  parseTorrentMetadata,
  fetchTorrentMetadata,
  setTorrentCategory,
  addTorrentTags,
  removeTorrentTags,
  fetchCategories,
  fetchTags,
  disconnect,
  chooseFilter,
  chooseCategory,
} = useTorrentLibrary()

const addModal = useTemplateRef<AddTorrentModalApi>('addModal')
const autoSelectIds = ref<string[]>([])
const dropActive = ref(false)
let dragDepth = 0

const detailTorrentId = ref<string | null>(null)
const toggleDetails = (torrentId: string) => {
  detailTorrentId.value = detailTorrentId.value === torrentId ? null : torrentId
}

const instanceCategories = ref<string[]>([])
const instanceTags = ref<string[]>([])

const loadInstanceCollections = async () => {
  const [fetchedCategories, fetchedTags] = await Promise.all([fetchCategories(), fetchTags()])
  if (fetchedCategories) instanceCategories.value = fetchedCategories
  if (fetchedTags) instanceTags.value = fetchedTags
}

const onSetCategory = async (torrentIds: string[], category: string) => {
  if (await setTorrentCategory(torrentIds, category)) void loadInstanceCollections()
}

const onAddTags = async (torrentIds: string[], tags: string[]) => {
  if (await addTorrentTags(torrentIds, tags)) void loadInstanceCollections()
}

const onRemoveTags = async (torrentIds: string[], tags: string[]) => {
  if (await removeTorrentTags(torrentIds, tags)) void loadInstanceCollections()
}

const settingsOpen = ref(false)
const authenticationMode = ref<AuthenticationMode>('apiKey')
const connectionForm = reactive({
  endpoint: 'http://localhost:8080',
  apiKey: '',
  username: '',
  password: '',
})

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
  if (connectionStatus.value === 'connected') return { label: 'Connected', color: 'success' as const }
  if (connectionStatus.value === 'connecting') return { label: 'Connecting', color: 'neutral' as const }
  return { label: 'Disconnected', color: 'error' as const }
})

const connectionTarget = computed(() => connectionEndpoint.value || savedProfile.value?.endpoint || '')
const connectionHost = computed(() => {
  if (!connectionTarget.value) return ''
  try {
    return new URL(connectionTarget.value).host
  }
  catch {
    return connectionTarget.value
  }
})

const connectionDescription = computed(() => {
  if (stale.value) return connectionHost.value ? `Last reached ${connectionHost.value}` : 'Torrent data may be out of date'
  if (connectionStatus.value === 'connected') return connectionHost.value
  if (connectionStatus.value === 'connecting') return connectionHost.value ? `Connecting to ${connectionHost.value}` : 'Resolving connection'
  return connectionHost.value ? `Saved: ${connectionHost.value}` : 'No instance configured'
})

const connectionDotClass = computed(() => {
  if (stale.value) return 'bg-warning'
  if (connectionStatus.value === 'connected') return 'bg-success'
  if (connectionStatus.value === 'connecting') return 'animate-pulse bg-muted'
  return 'bg-error'
})

const torrentActionsDisabled = computed(() => connectionStatus.value !== 'connected' || stale.value)

const canBrowseFolders = computed(() => typeof window !== 'undefined'
  && '__TAURI_INTERNALS__' in window
  && connectionStatus.value === 'connected'
  && !stale.value
  && isLoopbackEndpoint(connectionEndpoint.value))

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

  const restored = await retry()

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

const updateTorrentActivity = async (torrentIds: string[], paused: boolean) => {
  const successful = await setTorrentsPaused(torrentIds, paused)
  const action = paused ? 'stopped' : 'started'

  if (successful) {
    toast.add({
      title: `${torrentIds.length === 1 ? 'Torrent' : 'Torrents'} ${action}`,
      description: `${torrentIds.length} selected torrent${torrentIds.length === 1 ? '' : 's'} ${action}.`,
      color: 'success',
    })
    return
  }

  toast.add({
    title: `Could not ${paused ? 'stop' : 'start'} ${torrentIds.length === 1 ? 'torrent' : 'torrents'}`,
    description: torrentActionError.value || 'The torrent action is unavailable while qBittorrent is disconnected.',
    color: 'error',
  })
}

const removeSelectedTorrents = async (torrentIds: string[], deleteFiles: boolean) => {
  const successful = await removeTorrents(torrentIds, deleteFiles)
  const noun = torrentIds.length === 1 ? 'Torrent' : 'Torrents'

  if (successful) {
    toast.add({
      title: `${noun} removed`,
      description: deleteFiles
        ? `${torrentIds.length} torrent${torrentIds.length === 1 ? '' : 's'} removed and the downloaded files were deleted.`
        : `${torrentIds.length} torrent${torrentIds.length === 1 ? '' : 's'} removed. The downloaded content was kept.`,
      color: 'success',
    })
    return
  }

  toast.add({
    title: `Could not remove ${noun.toLowerCase()}`,
    description: torrentActionError.value || 'Torrents cannot be removed while qBittorrent is disconnected.',
    color: 'error',
  })
}

const openAddModal = () => {
  void loadDefaultSavePath()
  addModal.value?.openWith()
}

const addTorrentsFromModal = async (input: AddTorrentsInput) => {
  const outcome = await addTorrents(input)

  if (!outcome) {
    toast.add({
      title: 'Could not add torrents',
      description: torrentActionError.value || 'Torrents cannot be added while qBittorrent is disconnected.',
      color: 'error',
    })
    return
  }

  const { successCount, failureCount, pendingCount, addedTorrentIds } = outcome

  if (!successCount && !pendingCount) {
    toast.add({
      title: 'qBittorrent added nothing',
      description: 'Every source was rejected — usually duplicates or unreachable URLs.',
      color: 'error',
    })
    return
  }

  addModal.value?.close()
  if (addedTorrentIds.length) autoSelectIds.value = addedTorrentIds

  if (successCount) {
    toast.add({
      title: successCount === 1 ? 'Torrent added' : `${successCount} torrents added`,
      description: pendingCount
        ? `${pendingCount} more ${pendingCount === 1 ? 'source is' : 'sources are'} still being fetched by qBittorrent.`
        : undefined,
      color: 'success',
    })
  }
  else {
    toast.add({
      title: 'qBittorrent is fetching the torrent',
      description: 'The library updates once the metadata arrives.',
      color: 'info',
    })
  }

  if (failureCount) {
    toast.add({
      title: failureCount === 1 ? 'One source was rejected' : `${failureCount} sources were rejected`,
      description: 'Usually duplicates already in the library or unreachable URLs.',
      color: 'warning',
    })
  }
}

const extractMagnets = (text: string) => text
  .split(/\r?\n/)
  .map(line => line.trim())
  .filter(line => line.toLowerCase().startsWith('magnet:'))

const onDragEnter = (event: DragEvent) => {
  const types = Array.from(event.dataTransfer?.types ?? [])
  if (!types.includes('Files') && !types.includes('text/uri-list') && !types.includes('text/plain')) return
  dragDepth += 1
  dropActive.value = true
}

const onDragLeave = () => {
  dragDepth = Math.max(0, dragDepth - 1)
  if (!dragDepth) dropActive.value = false
}

const onDrop = (event: DragEvent) => {
  dragDepth = 0
  dropActive.value = false

  const torrentFiles = Array.from(event.dataTransfer?.files ?? [])
    .filter(file => file.name.toLowerCase().endsWith('.torrent'))
  // getData returns an empty string for a missing type, so the fallbacks
  // must be chained with || rather than ??.
  const draggedText = event.dataTransfer?.getData('text/uri-list')
    || event.dataTransfer?.getData('text')
    || event.dataTransfer?.getData('text/plain')
    || ''
  const magnets = extractMagnets(draggedText)
  if (!torrentFiles.length && !magnets.length) return

  void loadDefaultSavePath()
  addModal.value?.openWith({ urls: magnets, files: torrentFiles })
}

const acceptIncomingUrl = (url: string) => {
  try {
    if (new URL(url).protocol !== 'magnet:') return
  }
  catch {
    return
  }
  void loadDefaultSavePath()
  addModal.value?.openWith({ urls: [url] })
}

const listenForMagnetLinks = async () => {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return

  try {
    const { getCurrent, onOpenUrl } = await import('@tauri-apps/plugin-deep-link')
    onOpenUrl(urls => urls?.forEach(acceptIncomingUrl))
    const current = await getCurrent()
    current?.forEach(acceptIncomingUrl)
  }
  catch {
    // Deep links are unavailable before the OS registers the scheme.
  }
}

const magnetHintOpen = ref(false)
const magnetHintKind = ref<MagnetHandlerStatus>('otherProgram')

const checkMagnetHandler = async () => {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return
  if (!navigator.userAgent.includes('Windows')) return

  try {
    const status = await invoke<MagnetHandlerStatus>('magnet_handler_status')
    if (status === 'cloudburstDefault') return
    magnetHintKind.value = status
    magnetHintOpen.value = true
  }
  catch {
    // Handler detection is best-effort; adding torrents works regardless.
  }
}

const openMagnetSettings = () => {
  magnetHintOpen.value = false
  void invoke('open_default_apps_settings')
}

watch(connectionStatus, (status) => {
  if (status === 'connected') {
    void loadDefaultSavePath()
    void loadInstanceCollections()
    return
  }
  if (status === 'disconnected') detailTorrentId.value = null
})

let stopAutoRefresh: (() => void) | undefined

onMounted(() => {
  void restoreSavedConnection()
  stopAutoRefresh = startAutoRefresh()
  void listenForMagnetLinks()
  void checkMagnetHandler()
})

onBeforeUnmount(() => {
  stopAutoRefresh?.()
})
</script>

<template>
  <div @dragover.prevent @dragenter.prevent="onDragEnter" @dragleave.prevent="onDragLeave" @drop.prevent="onDrop">
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
        <button
          type="button"
          class="w-full space-y-2.5 rounded-lg border border-transparent p-2 text-left text-sm transition-colors hover:border-default hover:bg-elevated focus-visible:outline-2 focus-visible:outline-primary"
          :title="connectionTarget || 'Configure a qBittorrent connection'"
          @click="settingsOpen = true"
        >
          <span class="flex items-center justify-between gap-2">
            <span class="flex min-w-0 items-center gap-2 text-highlighted">
              <span class="size-2 shrink-0 rounded-full" :class="connectionDotClass" />
              <span class="truncate font-medium">qBittorrent</span>
            </span>
            <UBadge :label="connectionBadge.label" :color="connectionBadge.color" variant="subtle" size="sm" />
          </span>
          <span class="flex min-w-0 items-center justify-between gap-2 text-xs text-muted">
            <span class="truncate">{{ connectionDescription }}</span>
            <span v-if="connectionStatus === 'connected' && connectionVersion" class="shrink-0 font-mono">v{{ connectionVersion }}</span>
          </span>
          <span v-if="connectionStatus === 'connected' && !stale" class="flex items-center justify-between font-mono text-xs text-muted">
            <span>↓ {{ formatSpeed(transferTotals.down) }}</span>
            <span>↑ {{ formatSpeed(transferTotals.up) }}</span>
          </span>
        </button>
      </template>
    </UDashboardSidebar>

    <UDashboardPanel id="torrent-list" :ui="{ body: 'gap-0 overflow-hidden p-0 sm:p-0' }">
      <template #body>
        <TorrentTable
          :torrents="visibleTorrents"
          :actions-disabled="torrentActionsDisabled"
          :action-pending="activityUpdating"
          :auto-select-ids="autoSelectIds"
          :categories="instanceCategories"
          :tags="instanceTags"
          :open-torrent-id="detailTorrentId"
          @set-paused="updateTorrentActivity"
          @remove-torrents="removeSelectedTorrents"
          @set-category="onSetCategory"
          @add-tags="onAddTags"
          @remove-tags="onRemoveTags"
          @toggle-details="toggleDetails"
        >
          <template #actions>
            <UTooltip text="Add torrents">
              <UButton
                icon="i-lucide-plus"
                color="primary"
                variant="soft"
                size="sm"
                aria-label="Add torrents"
                :disabled="torrentActionsDisabled"
                :loading="activityUpdating"
                @click="openAddModal"
              />
            </UTooltip>
            <UTooltip v-if="connectionEndpoint" text="Refresh torrents">
              <UButton
                icon="i-lucide-refresh-cw"
                color="neutral"
                variant="ghost"
                size="sm"
                aria-label="Refresh torrents"
                :loading="refreshing"
                :disabled="activityUpdating"
                @click="retryConnection"
              />
            </UTooltip>
            <UTooltip :text="connectionEndpoint ? 'Connection settings' : 'Connect qBittorrent'">
              <UButton
                :icon="connectionEndpoint ? 'i-lucide-settings' : 'i-lucide-plug-zap'"
                color="neutral"
                variant="ghost"
                size="sm"
                :aria-label="connectionEndpoint ? 'Connection settings' : 'Connect qBittorrent'"
                @click="settingsOpen = true"
              />
            </UTooltip>
          </template>

          <template #notice>
            <div v-if="stale" class="mx-4 mt-4 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-warning/35 bg-warning/10 px-4 py-3 text-sm sm:mx-6 sm:mt-6">
              <div class="flex min-w-0 items-start gap-3">
                <UIcon name="i-lucide-cloud-off" class="mt-0.5 size-4 shrink-0 text-warning" />
                <div>
                  <p class="font-medium text-highlighted">Disconnected — showing stale torrents</p>
                  <p class="mt-0.5 text-muted">{{ connectionError }}</p>
                </div>
              </div>
              <UButton label="Reconnect" icon="i-lucide-refresh-cw" color="warning" variant="soft" size="sm" :loading="refreshing" @click="retryConnection" />
            </div>
          </template>

          <template #empty>
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
          </template>
        </TorrentTable>
      </template>
    </UDashboardPanel>

    <UDashboardPanel
      v-if="detailTorrentId"
      id="torrent-detail"
      resizable
      :default-size="35"
      :min-size="25"
      :max-size="50"
    >
      <TorrentDetailPanel
        :torrent-id="detailTorrentId"
        @close="detailTorrentId = null"
        @changed="loadInstanceCollections"
      />
    </UDashboardPanel>
    </UDashboardGroup>

    <Transition name="fade">
      <div v-if="dropActive" class="pointer-events-none fixed inset-0 z-50 flex items-center justify-center bg-primary/10 backdrop-blur-sm">
        <div class="flex flex-col items-center gap-2 rounded-xl border-2 border-dashed border-primary bg-default/90 px-10 py-8">
          <UIcon name="i-lucide-file-down" class="size-8 text-primary" />
          <p class="font-medium text-highlighted">Drop torrents to add them</p>
          <p class="text-sm text-muted">.torrent files or magnet links</p>
        </div>
      </div>
    </Transition>

    <AddTorrentModal
      ref="addModal"
      :categories="categories"
      :default-save-path="defaultSavePath"
      :can-browse="canBrowseFolders"
      :pending="activityUpdating"
      :parse-metadata="parseTorrentMetadata"
      :fetch-metadata="fetchTorrentMetadata"
      @add="addTorrentsFromModal"
    />

  <UModal
    v-model:open="magnetHintOpen"
    title="Cloudburst is not your magnet link handler"
    description="Windows routes magnet links to the program chosen in the system's default apps."
  >
    <template #body>
      <div class="space-y-3 text-sm text-muted">
        <p>
          Cloudburst registers itself automatically, but a default program chosen in Windows
          Settings always takes precedence.
        </p>
        <ol class="list-decimal space-y-1 ps-5">
          <li>Open Windows Settings → Apps → Default apps.</li>
          <li>Search for “magnet”.</li>
          <li>Choose <span class="text-highlighted">Cloudburst</span>.</li>
        </ol>
        <p>Magnet links clicked in your browser will then open the add dialog here.</p>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-end gap-2">
        <UButton type="button" label="Not now" color="neutral" variant="ghost" @click="magnetHintOpen = false" />
        <UButton type="button" label="Open Settings" icon="i-lucide-settings" @click="openMagnetSettings" />
      </div>
    </template>
  </UModal>

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
  </div>
</template>
