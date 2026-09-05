<script setup lang="ts">
import type { AddContentLayout, AddTorrentFile, AddTorrentResult, AddTorrentsBatchOutcome, AddTorrentsInput, MetadataFetch, TorrentMetadata } from '~/types/torrent'
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

const rememberedSavePathStorageKey = 'cloudburst:last-save-path'
const open = ref(false)
const step = ref<'sources' | 'review' | 'results'>('sources')
const submissionError = ref('')
const preparing = ref(false)
const busy = computed(() => props.pending || preparing.value)
const submittedSources = ref<SingleSource[]>([])
const waitingSources = ref<{ urls: string[], files: File[] }>({ urls: [], files: [] })
const waitingSourceCount = computed(() => waitingSources.value.urls.length + waitingSources.value.files.length)
const results = ref<AddTorrentResult[]>([])
const resultLabels: Record<AddTorrentResult['status'], string> = {
  added: 'Added', rejected: 'Rejected', pending: 'Still fetching', unknown: 'Result unknown', notSubmitted: 'Not submitted',
}
const resultColors = { added: 'success', rejected: 'error', pending: 'info', unknown: 'warning', notSubmitted: 'neutral' } as const
const canEditResult = (result: AddTorrentResult) => result.status === 'rejected' || result.status === 'notSubmitted'
const hasFailedSources = computed(() => results.value.some(canEditResult))
const hasPendingSources = computed(() => results.value.some(result => result.status === 'pending'))
const hasUnknownSources = computed(() => results.value.some(result => result.status === 'unknown'))
const sourceLabel = (source: SingleSource | undefined) => source?.kind === 'file' ? source.file.name : source?.url
const urlsText = ref('')
const chosenFiles = ref<ChosenFile[]>([])
const category = ref('')
const savePath = ref('')
const rememberSavePath = ref(false)
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
const modalTitle = computed(() => step.value === 'results' ? 'Add results' : (isReview.value ? (sourceCount.value === 1 ? 'Review torrent' : 'Review torrents') : 'Add torrents'))
const modalDescription = computed(() => step.value === 'results'
  ? 'Review what happened to each source.' : isReview.value
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

const emitAdd = (input: AddTorrentsInput) => {
  const path = savePath.value.trim()
  if (rememberSavePath.value && path) localStorage.setItem(rememberedSavePathStorageKey, path)
  else localStorage.removeItem(rememberedSavePathStorageKey)
  submittedSources.value = singleSource.value ? [singleSource.value] : [
    ...parseUrlList().map(url => ({ kind: 'url' as const, url })),
    ...chosenFiles.value.map(file => ({ kind: 'file' as const, file })),
  ]
  clearPolling()
  emit('add', input)
}

const showOutcome = (outcome: AddTorrentsBatchOutcome | null, error?: string) => {
  if (!outcome) {
    submissionError.value = error || 'Could not add torrents. Your sources and settings have been kept.'
    return
  }
  if (outcome.results.length && outcome.results.every(result => result.status === 'added')) {
    close()
    return
  }
  results.value = outcome.results
  step.value = 'results'
  resetMetadataState()
}

const editFailedSources = () => {
  const failed = submittedSources.value.filter((_, index) => results.value[index] && canEditResult(results.value[index]!))
  urlsText.value = failed.flatMap(source => source.kind === 'url' ? [source.url] : []).join('\n')
  chosenFiles.value = failed.flatMap(source => source.kind === 'file' ? [source.file] : [])
  results.value = []
  submissionError.value = ''
  backToSources()
}

const resetForm = () => {
  step.value = 'sources'
  results.value = []
  submittedSources.value = []
  submissionError.value = ''
  urlsText.value = ''
  chosenFiles.value = []
  category.value = ''
  savePath.value = localStorage.getItem(rememberedSavePathStorageKey) || ''
  rememberSavePath.value = Boolean(savePath.value)
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
  if (open.value && (busy.value || step.value === 'results')) {
    waitingSources.value.urls.push(...(options.urls ?? []))
    waitingSources.value.files.push(...(options.files ?? []))
    return
  }
  resetForm()
  options.urls?.forEach(appendUrl)
  if (options.files?.length) addFiles(options.files)
  open.value = true
}

const close = () => {
  open.value = false
  clearPolling()
  if (waitingSourceCount.value) {
    const sources = waitingSources.value
    waitingSources.value = { urls: [], files: [] }
    openWith(sources)
  }
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
  if (busy.value || !sourceCount.value) return

  submissionError.value = ''
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

  try {
    const base64Content = await fileToBase64(source.file.file)
    if (singleSource.value !== source || !open.value) return
    const parsed = await props.parseMetadata([{ name: source.file.name, base64Content }])
    if (singleSource.value !== source || !open.value) return
    metadata.value = parsed?.[0] ?? null
    metadataFailed.value = !metadata.value
  }
  catch {
    if (singleSource.value === source) metadataFailed.value = true
  }
  finally {
    if (singleSource.value === source) metadataLoading.value = false
  }
}

const startPolling = (source: string) => {
  metadataLoading.value = true
  metadataFailed.value = false

  const tick = async () => {
    const result = await props.fetchMetadata(source)
    if (busy.value || step.value !== 'review' || singleSource.value?.kind !== 'url' || singleSource.value.url !== source || !open.value) return

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

const prepareSubmission = async () => {
  const urls = parseUrlList()

  if (singleSource.value) {
    const partialSelection = treeSelection.value !== null && !treeSelection.value.allSelected

    if (singleSource.value.kind === 'url') {
      emitAdd({
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
      emitAdd({
        urls: [hash],
        files: [],
        ...commonOptions(),
        filePriorities: partialSelection ? treeSelection.value!.priorities : undefined,
      })
      return
    }

    // The instance could not read the file; fall back to a plain upload.
    emitAdd({ urls: [], files: [{ name: source.file.name, base64Content }], ...commonOptions() })
    return
  }

  const files: AddTorrentFile[] = []
  for (const chosen of chosenFiles.value) {
    files.push({ name: chosen.name, base64Content: await fileToBase64(chosen.file) })
  }
  emitAdd({ urls, files, ...commonOptions() })
}

const submit = async () => {
  if (busy.value || !sourceCount.value) return
  preparing.value = true
  submissionError.value = ''
  try {
    await prepareSubmission()
  }
  catch (error) {
    submissionError.value = `Could not prepare torrent files: ${error instanceof Error ? error.message : String(error)}`
  }
  finally {
    preparing.value = false
  }
}

onBeforeUnmount(clearPolling)

defineExpose({ openWith, close, showOutcome })
</script>

<template>
  <UModal
    v-model:open="open"
    :title="modalTitle"
    :description="modalDescription"
    :dismissible="!busy"
    :close="busy ? false : undefined"
    :ui="{ content: treePaneVisible ? 'max-w-7xl' : 'max-w-3xl' }"
  >
    <template #body>
      <p v-if="waitingSourceCount" class="mb-4 text-sm text-muted">{{ waitingSourceCount }} incoming {{ waitingSourceCount === 1 ? 'source is' : 'sources are' }} waiting for review after this batch.</p>
      <div v-if="submissionError" role="alert" class="mb-4 rounded-md border border-error/30 bg-error/10 px-3 py-2.5 text-sm text-error">{{ submissionError }}</div>
      <div v-if="step === 'results'" class="space-y-4" aria-live="polite">
        <ul class="max-h-80 divide-y divide-default overflow-y-auto" aria-label="Source results">
          <li v-for="(result, index) in results" :key="index" class="py-3 first:pt-0">
            <div class="flex items-start justify-between gap-3">
              <span class="min-w-0 break-all text-sm text-highlighted">{{ sourceLabel(submittedSources[index]) }}</span>
              <UBadge :label="resultLabels[result.status]" :color="resultColors[result.status]" variant="subtle" class="shrink-0" />
            </div>
            <p v-if="result.message || result.status === 'rejected'" class="mt-1 break-words text-xs text-muted">{{ result.message || 'qBittorrent rejected this source without a specific reason.' }}</p>
          </li>
        </ul>
        <p v-if="hasPendingSources" class="text-sm text-muted">Pending sources were accepted for fetching. They will appear in the library once resolved; you can close this dialog.</p>
        <p v-if="hasUnknownSources" class="text-sm text-muted">Some results could not be confirmed. Check the library before adding those sources again; qBittorrent may already have accepted them.</p>
      </div>
      <div v-else-if="step === 'sources'" class="space-y-5">
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
              :disabled="busy"
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
                  :disabled="busy"
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

      <fieldset v-else :disabled="busy" class="min-w-0">
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
                  :disabled="busy"
                  @click="browseForFolder"
                />
              </div>
              <div class="mt-2 flex items-center justify-between gap-3">
                <span class="text-sm text-highlighted">Remember this location for future torrents</span>
                <USwitch v-model="rememberSavePath" aria-label="Remember this location for future torrents" />
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
      </fieldset>
    </template>

    <template #footer>
      <div v-if="step === 'results'" class="flex w-full items-center justify-end gap-2">
        <UButton v-if="hasFailedSources" label="Edit failed sources" color="neutral" variant="outline" @click="editFailedSources" />
        <UButton label="Done" @click="close" />
      </div>
      <div v-else class="flex w-full items-center justify-end gap-2">
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
          :disabled="busy"
          @click="backToSources"
        />
        <UButton
          v-if="step === 'sources'"
          type="button"
          label="Continue"
          icon="i-lucide-arrow-right"
          trailing
          :disabled="!sourceCount || busy"
          @click="continueToReview"
        />
        <UButton
          v-else
          type="button"
          :label="submitLabel"
          icon="i-lucide-plus"
          :loading="busy"
          :disabled="!sourceCount"
          @click="submit"
        />
      </div>
    </template>
  </UModal>
</template>
