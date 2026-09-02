<script setup lang="ts">
import type { AddContentLayout, TorrentMetadataFile } from '~/types/torrent'
import { formatBytes } from '~/utils/torrent-format'
import {
  buildFileTree,
  commonRootFolder,
  fileIconFor,
  priorityForRating,
  priorityLabel,
  ratingForPriority,
  stripRootFolder,
  type TorrentFileTreeNode,
  type TorrentTreeFileInput,
} from '~/utils/torrent-file-tree'

const props = defineProps<{
  files: TorrentMetadataFile[] | TorrentTreeFileInput[]
  layout?: AddContentLayout
  folderName?: string
  priorities?: number[]
  resetKey?: string | number
}>()

const emit = defineEmits<{
  change: [payload: { priorities: number[], selectedSize: number, allSelected: boolean }]
}>()

const selection = ref<Record<number, number>>({})
const collapsedFolders = ref<Set<string>>(new Set())

const treeRoots = computed(() => {
  if (props.layout === 'subfolder') {
    const children = buildFileTree(props.files)
    if (commonRootFolder(props.files)) return children
    return [{
      name: props.folderName || 'Torrent',
      path: props.folderName || 'Torrent',
      isFolder: true,
      size: children.reduce((total, node) => total + node.size, 0),
      fileIndex: null,
      progress: null,
      children,
    }]
  }
  if (props.layout === 'noSubfolder') return buildFileTree(stripRootFolder(props.files))
  return buildFileTree(props.files)
})
const totalSize = computed(() => props.files.reduce((total, file) => total + file.length, 0))

const priorityAt = (index: number) => selection.value[index] ?? props.priorities?.[index] ?? 1

const isLeafKept = (index: number) => priorityAt(index) >= 1

const selectedSize = computed(() => props.files.reduce(
  (total, file, index) => (isLeafKept(index) ? total + file.length : total),
  0,
))

const leafIndexes = (node: TorrentFileTreeNode, indexes: number[] = []): number[] => {
  if (!node.isFolder && node.fileIndex !== null) indexes.push(node.fileIndex)
  node.children.forEach(child => leafIndexes(child, indexes))
  return indexes
}

const nodeState = (node: TorrentFileTreeNode): boolean | 'indeterminate' => {
  const leaves = leafIndexes(node)
  const kept = leaves.filter(isLeafKept).length
  if (!kept) return false
  return kept === leaves.length || 'indeterminate'
}

const setLeaves = (node: TorrentFileTreeNode, priority: number) => {
  const next = { ...selection.value }
  leafIndexes(node).forEach((index) => {
    next[index] = priority
  })
  selection.value = next
}

const toggleNode = (node: TorrentFileTreeNode, value: boolean | 'indeterminate') => {
  setLeaves(node, value === true ? 1 : 0)
  emitChange()
}

const setFileRating = (node: TorrentFileTreeNode, rating: number) => {
  if (node.fileIndex === null) return
  selection.value = { ...selection.value, [node.fileIndex]: priorityForRating(rating) }
  emitChange()
}

const selectAll = (value: boolean) => {
  setLeaves({ name: '', path: '', isFolder: true, size: 0, fileIndex: null, progress: null, children: treeRoots.value }, value ? 1 : 0)
  emitChange()
}

const toggleFolderCollapsed = (node: TorrentFileTreeNode) => {
  const next = new Set(collapsedFolders.value)
  if (next.has(node.path)) next.delete(node.path)
  else next.add(node.path)
  collapsedFolders.value = next
}

const isCollapsed = (node: TorrentFileTreeNode) => collapsedFolders.value.has(node.path)

const visibleRows = computed(() => {
  const rows: Array<{ node: TorrentFileTreeNode, depth: number }> = []
  const walk = (nodes: TorrentFileTreeNode[], depth: number) => nodes.forEach((node) => {
    rows.push({ node, depth })
    if (node.isFolder && !isCollapsed(node)) walk(node.children, depth + 1)
  })
  walk(treeRoots.value, 0)
  return rows
})

const emitChange = () => {
  const priorities = props.files.map((_, index) => (isLeafKept(index) ? priorityAt(index) : 0))
  emit('change', {
    priorities,
    selectedSize: selectedSize.value,
    allSelected: priorities.every(priority => priority >= 1),
  })
}

watch(() => props.files, () => {
  if (props.resetKey === undefined) {
    selection.value = {}
    collapsedFolders.value = new Set()
  }
  emitChange()
}, { immediate: true })

watch(() => props.resetKey, () => {
  selection.value = {}
  collapsedFolders.value = new Set()
  emitChange()
})

watch(() => props.priorities, () => emitChange())
</script>

<template>
  <div class="flex min-h-0 flex-col gap-1">
    <div class="flex items-center justify-between gap-2">
      <p class="text-sm font-medium text-highlighted">
        {{ selectedSize === totalSize ? formatBytes(totalSize) : `${formatBytes(selectedSize)} of ${formatBytes(totalSize)}` }}
      </p>
      <div class="flex items-center gap-1">
        <UButton type="button" label="All" color="neutral" variant="outline" size="xs" aria-label="Set all files to normal priority" @click="selectAll(true)" />
        <UButton type="button" label="None" color="neutral" variant="outline" size="xs" aria-label="Skip all files" @click="selectAll(false)" />
      </div>
    </div>

    <ul class="max-h-128 min-h-0 space-y-0.5 overflow-y-auto pe-1">
      <li v-for="row in visibleRows" :key="row.node.path">
        <div
          class="flex items-center gap-1.5 rounded px-1 py-0.5 hover:bg-elevated"
          :style="{ paddingInlineStart: `${row.depth * 18 + 4}px` }"
        >
          <button
            v-if="row.node.isFolder"
            type="button"
            class="flex shrink-0"
            :aria-label="`${isCollapsed(row.node) ? 'Expand' : 'Collapse'} ${row.node.name}`"
            @click="toggleFolderCollapsed(row.node)"
          >
            <UIcon :name="isCollapsed(row.node) ? 'i-lucide-chevron-right' : 'i-lucide-chevron-down'" class="size-4 text-muted" />
          </button>
          <span v-else class="w-4 shrink-0" />
          <UCheckbox
            v-if="row.node.isFolder"
            :model-value="nodeState(row.node)"
            :aria-label="`Include ${row.node.name}`"
            @update:model-value="toggleNode(row.node, $event)"
          />
          <UInputRating
            v-else
            :model-value="ratingForPriority(priorityAt(row.node.fileIndex!))"
            :length="3"
            clearable
            hoverable
            empty-icon="ph-caret-circle-double-down"
            icon="ph-caret-circle-double-down-fill"
            :aria-label="`Priority for ${row.node.name}`"
            :title="priorityLabel(priorityAt(row.node.fileIndex!))"
            @update:model-value="setFileRating(row.node, $event)"
          />
          <UIcon :name="row.node.isFolder ? 'i-lucide-folder' : fileIconFor(row.node.path)" class="size-4 shrink-0 text-muted" />
          <span class="min-w-0 flex-1 truncate text-sm">{{ row.node.name }}</span>
          <span
            v-if="row.node.progress !== null && row.node.progress < 1"
            class="shrink-0 font-mono text-xs text-muted"
          >{{ Math.round(row.node.progress * 100) }}%</span>
          <span class="shrink-0 font-mono text-xs text-muted">{{ formatBytes(row.node.size) }}</span>
        </div>
      </li>
    </ul>
  </div>
</template>
