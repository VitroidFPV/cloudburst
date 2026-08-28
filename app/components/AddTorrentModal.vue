<script setup lang="ts">
import type { AddContentLayout, AddTorrentFile, AddTorrentsInput } from '~/types/torrent'
import { formatBytes } from '~/utils/torrent-format'
import { fileToBase64 } from '~/utils/torrent-file'

const props = defineProps<{
  categories: string[]
  defaultSavePath: string
  canBrowse: boolean
  pending: boolean
}>()

const emit = defineEmits<{
  add: [input: AddTorrentsInput]
}>()

interface ChosenFile {
  name: string
  size: number
  file: File
}

const open = ref(false)
const urlsText = ref('')
const chosenFiles = ref<ChosenFile[]>([])
const category = ref('')
const savePath = ref('')
const contentLayout = ref<AddContentLayout>('original')
const fileInput = useTemplateRef<HTMLInputElement>('fileInput')

const folderLayoutItems = [
  { label: "Torrent's own folders", value: 'original' },
  { label: 'Always create a new folder', value: 'subfolder' },
  { label: 'No folder', value: 'noSubfolder' },
] satisfies Array<{ label: string, value: AddContentLayout }>

const sourceCount = computed(() => urlsText.value.split(/\r?\n/).filter(line => line.trim()).length + chosenFiles.value.length)
const submitLabel = computed(() => sourceCount.value === 1 ? 'Add torrent' : `Add ${sourceCount.value} torrents`)
const saveLocationPlaceholder = computed(() => props.defaultSavePath ? `Default: ${props.defaultSavePath}` : 'Instance default save location')

const resetForm = () => {
  urlsText.value = ''
  chosenFiles.value = []
  category.value = ''
  savePath.value = ''
  contentLayout.value = 'original'
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

const submit = async () => {
  if (props.pending || !sourceCount.value) return

  const urls = urlsText.value.split(/\r?\n/).map(line => line.trim()).filter(Boolean)
  const files: AddTorrentFile[] = []
  try {
    for (const chosen of chosenFiles.value) {
      files.push({ name: chosen.name, base64Content: await fileToBase64(chosen.file) })
    }
  }
  catch {
    return
  }

  emit('add', {
    urls,
    files,
    category: category.value.trim() || undefined,
    savePath: savePath.value.trim() || undefined,
    contentLayout: contentLayout.value,
  })
}

defineExpose({ openWith, close })
</script>

<template>
  <UModal v-model:open="open" title="Add torrents" description="Submit magnet links, URLs, or .torrent files to the active qBittorrent instance." :ui="{content: 'max-w-4xl'}">
    <template #body>
      <div class="space-y-5">
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
              <li v-for="chosen in chosenFiles" :key="chosen.name" class="flex items-center justify-between gap-2 rounded-md bg-elevated/50 px-2.5 py-1.5 text-sm">
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

        <div class="grid gap-4 sm:grid-cols-2">
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
            <USelect v-model="contentLayout" class="w-full" :items="folderLayoutItems" aria-label="Folder layout" />
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

        <p class="text-xs text-muted">
          Category, folder layout, and save location apply to every item added at once. qBittorrent reports the result after processing.
        </p>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full items-center justify-end gap-2">
        <UButton type="button" label="Cancel" color="neutral" variant="ghost" @click="close" />
        <UButton
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
