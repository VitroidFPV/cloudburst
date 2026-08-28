<script setup lang="ts">
import type { NavigationMenuItem, TableColumn } from '@nuxt/ui'
import type { AuthenticationMode, ConnectionInput, Torrent, TorrentStatus } from '~/types/torrent'
import { formatBytes, formatEta, formatSpeed, statusIcon, statusLabel } from '~/utils/torrent-format'

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
    header: 'Torrent',
    cell: ({ row }) => h('div', { class: 'flex max-w-72 min-w-0 items-center gap-3' }, [
      h(UIcon, { name: statusIcon[row.original.status], class: 'size-4 shrink-0 text-muted' }),
      h('div', { class: 'min-w-0' }, [
        h('p', { class: 'truncate font-medium text-highlighted' }, row.original.name),
        h('p', { class: 'truncate text-xs text-muted' }, `${row.original.category || 'Uncategorized'} · ${formatBytes(row.original.size)}`),
      ]),
    ]),
  },
  {
    accessorKey: 'progress',
    header: 'Progress',
    cell: ({ row }) => h('div', { class: 'flex min-w-24 items-center gap-2' }, [
      h(UProgress, { modelValue: row.original.progress, size: 'xs', class: 'w-16' }),
      h('span', { class: 'w-10 text-right font-mono text-xs text-muted' }, `${row.original.progress}%`),
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
    accessorKey: 'etaSeconds',
    header: 'ETA',
    cell: ({ row }) => h('span', { class: 'font-mono text-xs text-muted' }, formatEta(row.original.etaSeconds, row.original.status)),
  },
]

let refreshTimer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  void restoreSavedConnection()
  refreshTimer = setInterval(() => {
    if (connectionStatus.value === 'connected') void refresh()
  }, 5_000)
})

onBeforeUnmount(() => {
  if (refreshTimer) clearInterval(refreshTimer)
})
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

        <UTable v-if="visibleTorrents.length" :data="visibleTorrents" :columns="columns" class="min-h-0 flex-1" />

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
