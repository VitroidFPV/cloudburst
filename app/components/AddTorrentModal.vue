<script setup lang="ts">
import type { AddContentLayout, AddTorrentFile, AddTorrentsInput, MetadataFetch, TorrentMetadata } from '~/types/torrent'
import { formatBytes } from '~/utils/torrent-format'
import { fileToBase64 } from '~/utils/torrent-file'
import { commonRootFolder } from '~/utils/torrent-file-tree'

const props = defineProps<{
  categories: string[]
  defaultSavePath: string
  canBrowse: boolean
  pending: boolean
  parseMetadata: (files: AddTorrentFile[]) => Promise<TorrentMetadata[] | null>
  fetchMetadata: (source: string) => Promise<MetadataFetch | null>
}>()

const emit = defineEmits<{
  add: [input: AddTorrentsInput]
}>()

interface ChosenFile {
  name: string
  size: number
  file: File
}

type SingleSource = { kind: 'file', file: ChosenFile } | { kind: 'url', url: string }

const open = ref(false)
const step = ref<'sources' | 'review'>('sources')
const urlsText = ref('')
const chosenFiles = ref<ChosenFile[]>([])
const category = ref('')
const savePath = ref('')
const contentLayout = ref<AddContentLayout>('original')
const singleSource = ref<SingleSource | null>(null)
const metadata = ref<TorrentMetadata | null>(null)
const metadataLoading = ref(false)
const metadataFailed = ref(false)
const treeSelection = ref<{ priorities: number[], selectedSize: number, allSelected: boolean } | null>(null)
const fileInput = useTemplateRef<HTMLInputElement>('fileInput')
let pollTimer: ReturnType<typeof setTimeout> | undefined

const folderLayoutItems = [
  { label: 'Default', value: 'original', icon: 'i-lucide-folder-dot' },
  { label: 'Folder', value: 'subfolder', icon: 'i-lucide-folder-plus' },
  { label: 'No Folder', value: 'noSubfolder', icon: 'i-lucide-folder-x' },
] satisfies Array<{ label: string, value: AddContentLayout, icon: string }>

const sourceCount = computed(() => urlsText.value.split(/\r?\n/).filter(line => line.trim()).length + chosenFiles.value.length)
const isReview = computed(() => step.value === 'review')
const modalTitle = computed(() => (isReview.value ? (sourceCount.value === 1 ? 'Review torrent' : 'Review torrents') : 'Add torrents'))
const modalDescription = computed(() => isReview.value
  ? 'Choose where the torrent lands, then add it to qBittorrent.'
  : 'Submit magnet links, URLs, or .torrent files to the active qBittorrent instance.')
const submitLabel = computed(() => sourceCount.value === 1 ? 'Add torrent' : `Add ${sourceCount.value} torrents`)
const saveLocationPlaceholder = computed(() => props.defaultSavePath ? `Default: ${props.defaultSavePath}` : 'Instance default save location')
const treePaneVisible = computed(() => isReview.value && singleSource.value !== null)
const submittedContentLayout = computed<AddContentLayout>(() => (
  contentLayout.value === 'subfolder' && metadata.value && commonRootFolder(metadata.value.files)
    ? 'original'
    : contentLayout.value
))

const parseUrlList = () => urlsText.value.split(/\r?\n/).map(line => line.trim()).filter(Boolean)

const commonOptions = () => ({
  category: category.value.trim() || undefined,
  savePath: savePath.value.trim() || undefined,
  contentLayout: submittedContentLayout.value,
})

const resetForm = () => {
  step.value = 'sources'
  urlsText.value = ''
  chosenFiles.value = []
  category.value = ''
  savePath.value = ''
  contentLayout.value = 'original'
  resetMetadataState()
}

const resetMetadataState = () => {
  clearPolling()
  singleSource.value = null
  metadata.value = null
  metadataLoading.value = false
  metadataFailed.value = false
  treeSelection.value = null
}

const appendUrl = (url: string) => {
  const trimmed = url.trim()
  if (!trimmed || urlsText.value.split(/\r?\n/).some(line => line.trim() === trimmed)) return
  urlsText.value = urlsText.value ? `${urlsText.value}\n${trimmed}` : trimmed
}

const addFiles = (files: File[]) => {
  for (const file of files) {
    if (chosenFiles.value.some(chosen => chosen.name === file.name)) continue
    chosenFiles.value = [...chosenFiles.value, { name: file.name, size: file.size, file }]
  }
}

const openWith = (options: { urls?: string[], files?: File[] } = {}) => {
  resetForm()
  options.urls?.forEach(appendUrl)
  if (options.files?.length) addFiles(options.files)
  open.value = true
}

const close = () => {
  open.value = false
}

const chooseFiles = () => {
  fileInput.value?.click()
}

const onFilesChosen = (event: Event) => {
  const input = event.target as HTMLInputElement
  addFiles(Array.from(input.files ?? []))
  input.value = ''
}

const removeFile = (name: string) => {
  chosenFiles.value = chosenFiles.value.filter(chosen => chosen.name !== name)
}

const browseForFolder = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const path = await open({ directory: true, multiple: false })
    if (typeof path === 'string') savePath.value = path
  }
  catch {
    // The native folder dialog is an affordance, not a requirement.
  }
}

const continueToReview = () => {
  if (props.pending || !sourceCount.value) return

  step.value = 'review'
  resetMetadataState()

  const urls = parseUrlList()
  if (!urls.length && chosenFiles.value.length === 1) {
    singleSource.value = { kind: 'file', file: chosenFiles.value[0]! }
    void loadFileMetadata()
  }
  else if (urls.length === 1 && !chosenFiles.value.length) {
    singleSource.value = { kind: 'url', url: urls[0]! }
    startPolling(urls[0]!)
  }
}

const loadFileMetadata = async () => {
  const source = singleSource.value
  if (source?.kind !== 'file') return

  metadataLoading.value = true
  metadataFailed.value = false

  const base64Content = await fileToBase64(source.file.file)
  if (singleSource.value !== source || !open.value) return
  const parsed = await props.parseMetadata([{ name: source.file.name, base64Content }])
  if (singleSource.value !== source || !open.value) return

  metadata.value = parsed?.[0] ?? null
  metadataFailed.value = !metadata.value
  metadataLoading.value = false
}

const startPolling = (source: string) => {
  metadataLoading.value = true
  metadataFailed.value = false

  const tick = async () => {
    const result = await props.fetchMetadata(source)
    if (step.value !== 'review' || singleSource.value?.kind !== 'url' || singleSource.value.url !== source || !open.value) return

    if (result?.status === 'ready') {
      metadata.value = result.metadata
      metadataLoading.value = false
    }
    else if (result?.status === 'pending') {
      pollTimer = setTimeout(tick, 1_000)
    }
    else {
      metadataLoading.value = false
      metadataFailed.value = true
    }
  }
  void tick()
}

const clearPolling = () => {
  if (pollTimer) {
    clearTimeout(pollTimer)
    pollTimer = undefined
  }
}

const backToSources = () => {
  step.value = 'sources'
  resetMetadataState()
}

const onTreeChange = (payload: { priorities: number[], selectedSize: number, allSelected: boolean }) => {
  treeSelection.value = payload
}

const submit = async () => {
  if (props.pending || !sourceCount.value) return
  const urls = parseUrlList()

  if (singleSource.value) {
    const partialSelection = treeSelection.value !== null && !treeSelection.value.allSelected

    if (singleSource.value.kind === 'url') {
      emit('add', {
        urls: [singleSource.value.url],
        files: [],
        ...commonOptions(),
        filePriorities: partialSelection ? treeSelection.value!.priorities : undefined,
      })
      return
    }

    const source = singleSource.value
    const base64Content = await fileToBase64(source.file.file)
    const parsed = await props.parseMetadata([{ name: source.file.name, base64Content }])
    const hash = parsed?.[0]?.hash

    if (hash) {
      emit('add', {
        urls: [hash],
        files: [],
        ...commonOptions(),
        filePriorities: partialSelection ? treeSelection.value!.priorities : undefined,
      })
      return
    }

    // The instance could not read the file; fall back to a plain upload.
    emit('add', { urls: [], files: [{ name: source.file.name, base64Content }], ...commonOptions() })
    return
  }

  const files: AddTorrentFile[] = []
  try {
    for (const chosen of chosenFiles.value) {
      files.push({ name: chosen.name, base64Content: await fileToBase64(chosen.file) })
    }
  }
  catch {
    return
  }
  emit('add', { urls, files, ...commonOptions() })
}

onBeforeUnmount(clearPolling)

defineExpose({ openWith, close })
</script>

<template>
  <UModal
    v-model:open="open"
    :title="modalTitle"
    :description="modalDescription"
    :ui="{ content: treePaneVisible ? 'max-w-7xl' : 'max-w-3xl' }"
  >
    <template #body>
      <div v-if="step === 'sources'" class="space-y-5">
        <UFormField label="Magnet links or URLs" description="One magnet link or .torrent URL per line.">
          <UTextarea
            v-model="urlsText"
            class="w-full"
            :rows="3"
            aria-label="Magnet links or URLs"
            placeholder="magnet:?xt=urn:btih:…"
          />
        </UFormField>

        <UFormField label="Torrent files">
          <input
            ref="fileInput"
            type="file"
            accept=".torrent,application/x-bittorrent"
            multiple
            class="hidden"
            aria-label="Choose torrent files"
            @change="onFilesChosen"
          >
          <div class="space-y-2">
            <UButton
              type="button"
              label="Choose .torrent files"
              icon="i-lucide-file-plus-2"
              color="neutral"
              variant="outline"
              size="sm"
              aria-label="Choose torrent files"
              :disabled="pending"
              @click="chooseFiles"
            />
            <ul v-if="chosenFiles.length" class="space-y-1">
              <li v-for="chosen in chosenFiles" :key="chosen.name" class="flex items-center justify-between gap-2 rounded-md bg-elevated px-2.5 py-1.5 text-sm">
                <span class="min-w-0 flex-1 truncate">{{ chosen.name }}</span>
                <span class="shrink-0 font-mono text-xs text-muted">{{ formatBytes(chosen.size) }}</span>
                <UButton
                  type="button"
                  icon="i-lucide-x"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  :aria-label="`Remove ${chosen.name}`"
                  :disabled="pending"
                  @click="removeFile(chosen.name)"
                />
              </li>
            </ul>
          </div>
        </UFormField>

        <p class="text-xs text-muted">
          Adding a single torrent shows its file tree next, so you can pick which files to keep.
        </p>
      </div>

      <div v-else>
        <div :class="treePaneVisible ? 'grid items-start gap-5 lg:grid-cols-[minmax(18rem,2fr)_minmax(18rem,3fr)]' : ''">
          <div class="space-y-5 lg:pe-5">
            <div class="grid gap-4">
              <UFormField label="Category">
                <UInput
                  v-model="category"
                  class="w-full"
                  list="add-torrent-categories"
                  aria-label="Category"
                  placeholder="Optional — new names are created"
                />
                <datalist id="add-torrent-categories">
                  <option v-for="existing in categories" :key="existing" :value="existing" />
                </datalist>
              </UFormField>

              <UFormField label="Folder layout">
                <URadioGroup
                  v-model="contentLayout"
                  :items="folderLayoutItems"
                  variant="table"
                  orientation="horizontal"
                  indicator="hidden"
                  size="sm"
                  aria-label="Folder layout"
                  :ui="{ fieldset: 'grid grid-cols-3', item: 'justify-center text-center', wrapper: 'items-center' }"
                />
              </UFormField>
            </div>

            <UFormField label="Save location" :description="canBrowse ? 'Leave blank for the instance default. Pick a folder on the qBittorrent machine.' : 'Leave blank for the instance default. The path lives on the qBittorrent machine.'">
              <div class="flex gap-2">
                <UInput v-model="savePath" class="flex-1" :placeholder="saveLocationPlaceholder" aria-label="Save location" />
                <UButton
                  v-if="canBrowse"
                  type="button"
                  icon="i-lucide-folder-open"
                  color="neutral"
                  variant="outline"
                  size="sm"
                  aria-label="Browse for folder"
                  :disabled="pending"
                  @click="browseForFolder"
                />
              </div>
            </UFormField>

            <p v-if="!singleSource" class="text-xs text-muted">
              Category, folder layout, and save location apply to every item added at once. qBittorrent reports the result after processing.
            </p>
          </div>

          <div v-if="treePaneVisible" class="min-h-48 lg:border-s lg:border-default lg:ps-5">
            <div v-if="metadataLoading" class="flex h-full flex-col items-center justify-center gap-2 py-8 text-center">
              <UIcon name="i-lucide-loader-circle" class="size-6 animate-spin text-muted" />
              <p class="text-sm text-muted">Fetching metadata from qBittorrent…</p>
              <p class="text-xs text-muted">Large magnets can take a while. You can add the torrent without picking files.</p>
            </div>
            <div v-else-if="metadataFailed" class="flex h-full flex-col items-center justify-center gap-2 py-8 text-center">
              <UIcon name="i-lucide-circle-alert" class="size-6 text-warning" />
              <p class="text-sm text-highlighted">Metadata is unavailable</p>
              <p class="text-xs text-muted">The torrent can still be added without choosing files.</p>
            </div>
            <TorrentFileTree
              v-else-if="metadata"
              :files="metadata.files"
              :layout="contentLayout"
              :folder-name="metadata.name"
              @change="onTreeChange"
            />
          </div>
        </div>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-end gap-2">
        <UButton
          v-if="step === 'sources'"
          type="button"
          label="Cancel"
          color="neutral"
          variant="ghost"
          @click="close"
        />
        <UButton
          v-else
          type="button"
          label="Back"
          color="neutral"
          variant="ghost"
          :disabled="pending"
          @click="backToSources"
        />
        <UButton
          v-if="step === 'sources'"
          type="button"
          label="Continue"
          icon="i-lucide-arrow-right"
          trailing
          :disabled="!sourceCount || pending"
          @click="continueToReview"
        />
        <UButton
          v-else
          type="button"
          :label="submitLabel"
          icon="i-lucide-plus"
          :loading="pending"
          :disabled="!sourceCount"
          @click="submit"
        />
      </div>
    </template>
  </UModal>
</template>
