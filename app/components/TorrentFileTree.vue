<script setup lang="ts">
import type { AddContentLayout, TorrentMetadataFile } from '~/types/torrent'
import { formatBytes } from '~/utils/torrent-format'
import { buildFileTree, fileIconFor, stripRootFolder, type TorrentFileTreeNode } from '~/utils/torrent-file-tree'

const props = defineProps<{
  files: TorrentMetadataFile[]
  layout?: AddContentLayout
  folderName?: string
}>()

const emit = defineEmits<{
  change: [payload: { priorities: number[], selectedSize: number, allSelected: boolean }]
}>()

const selection = ref<Record<number, boolean>>({})
const collapsedFolders = ref<Set<string>>(new Set())

const treeRoots = computed(() => {
  if (props.layout === 'subfolder') {
    const children = buildFileTree(props.files)
    return [{
      name: props.folderName || 'Torrent',
      path: props.folderName || 'Torrent',
      isFolder: true,
      size: children.reduce((total, node) => total + node.size, 0),
      fileIndex: null,
      children,
    }]
  }
  if (props.layout === 'noSubfolder') return buildFileTree(stripRootFolder(props.files))
  return buildFileTree(props.files)
})
const totalSize = computed(() => props.files.reduce((total, file) => total + file.length, 0))

const isLeafSelected = (index: number) => selection.value[index] !== false

const selectedSize = computed(() => props.files.reduce(
  (total, file, index) => (isLeafSelected(index) ? total + file.length : total),
  0,
))

const leafIndexes = (node: TorrentFileTreeNode, indexes: number[] = []): number[] => {
  if (!node.isFolder && node.fileIndex !== null) indexes.push(node.fileIndex)
  node.children.forEach(child => leafIndexes(child, indexes))
  return indexes
}

const nodeState = (node: TorrentFileTreeNode): boolean | 'indeterminate' => {
  if (!node.isFolder) return isLeafSelected(node.fileIndex!)
  const leaves = leafIndexes(node)
  const selected = leaves.filter(isLeafSelected).length
  if (!selected) return false
  return selected === leaves.length || 'indeterminate'
}

const setLeaves = (node: TorrentFileTreeNode, value: boolean) => {
  const next = { ...selection.value }
  leafIndexes(node).forEach((index) => {
    next[index] = value
  })
  selection.value = next
}

const toggleNode = (node: TorrentFileTreeNode, value: boolean | 'indeterminate') => {
  setLeaves(node, value === true)
  emitChange()
}

const selectAll = (value: boolean) => {
  selection.value = Object.fromEntries(props.files.map((_, index) => [index, value]))
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
  const priorities = props.files.map((_, index) => (isLeafSelected(index) ? 1 : 0))
  emit('change', {
    priorities,
    selectedSize: selectedSize.value,
    allSelected: priorities.every(priority => priority === 1),
  })
}

watch(() => props.files, () => {
  selection.value = {}
  collapsedFolders.value = new Set()
  emitChange()
}, { immediate: true })
</script>

<template>
  <div class="flex min-h-0 flex-col gap-1">
    <div class="flex items-center justify-between gap-2">
      <p class="text-sm font-medium text-highlighted">
        {{ selectedSize === totalSize ? formatBytes(totalSize) : `${formatBytes(selectedSize)} of ${formatBytes(totalSize)}` }}
      </p>
      <div class="flex items-center gap-1">
        <UButton type="button" label="All" color="neutral" variant="ghost" size="xs" aria-label="Select all files" @click="selectAll(true)" />
        <UButton type="button" label="None" color="neutral" variant="ghost" size="xs" aria-label="Select no files" @click="selectAll(false)" />
      </div>
    </div>

    <ul class="max-h-128 min-h-0 space-y-0.5 overflow-y-auto pe-1">
      <li v-for="row in visibleRows" :key="row.node.path">
        <div
          class="flex items-center gap-1.5 rounded px-1 py-0.5 hover:bg-elevated/50"
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
            :model-value="nodeState(row.node)"
            :aria-label="`Include ${row.node.name}`"
            @update:model-value="toggleNode(row.node, $event)"
          />
          <UIcon :name="row.node.isFolder ? 'i-lucide-folder' : fileIconFor(row.node.path)" class="size-4 shrink-0 text-muted" />
          <span class="min-w-0 flex-1 truncate text-sm">{{ row.node.name }}</span>
          <span class="shrink-0 font-mono text-xs text-muted">{{ formatBytes(row.node.size) }}</span>
        </div>
      </li>
    </ul>
  </div>
</template>