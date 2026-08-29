import type { TorrentFilePriorityValue } from '../types/torrent'

export interface TorrentFileTreeNode {
  name: string
  path: string
  isFolder: boolean
  size: number
  fileIndex: number | null
  progress: number | null
  children: TorrentFileTreeNode[]
}

export interface TorrentTreeFileInput {
  path: string
  length: number
  progress?: number
}

const segmentPattern = /[\\/]/

export const buildFileTree = (files: TorrentTreeFileInput[]): TorrentFileTreeNode[] => {
  const roots: TorrentFileTreeNode[] = []

  const findOrCreate = (nodes: TorrentFileTreeNode[], name: string, isFolder: boolean, path: string) => {
    const existing = nodes.find(node => node.name === name && node.isFolder === isFolder)
    if (existing) return existing
    const node: TorrentFileTreeNode = { name, path, isFolder, size: 0, fileIndex: null, progress: null, children: [] }
    nodes.push(node)
    return node
  }

  files.forEach((file, fileIndex) => {
    const segments = file.path.split(segmentPattern).filter(Boolean)
    let level = roots
    segments.forEach((segment, depth) => {
      const isLeaf = depth === segments.length - 1
      const path = segments.slice(0, depth + 1).join('/')
      const node = findOrCreate(level, segment, !isLeaf, path)
      if (isLeaf) {
        node.fileIndex = fileIndex
        node.size = file.length
        node.progress = file.progress ?? null
      }
      level = node.children
    })
  })

  const computeFolderSizes = (nodes: TorrentFileTreeNode[]): number => nodes.reduce((total, node) => {
    if (node.isFolder) node.size = computeFolderSizes(node.children)
    return total + node.size
  }, 0)
  computeFolderSizes(roots)

  return roots
}

// qBittorrent per-file priorities: 0 skip, 1 normal, 6 high, 7 maximum.
// The rating control maps its three steps onto everything above skip.
export const priorityValues: readonly TorrentFilePriorityValue[] = [0, 1, 6, 7]

export const ratingForPriority = (priority: number): number => {
  const index = priorityValues.indexOf(priority as TorrentFilePriorityValue)
  return index >= 0 ? index : 1
}

export const priorityForRating = (rating: number): TorrentFilePriorityValue =>
  priorityValues[Math.min(Math.max(Math.round(rating), 0), priorityValues.length - 1)]!

export const priorityLabel = (priority: number): string =>
  ({ 0: 'Skipped', 1: 'Normal', 6: 'High', 7: 'Maximum' })[priority] ?? 'Normal'

const genericFileIcon = 'i-lucide-file'

const iconByExtension = new Map([
  ['mkv', 'i-lucide-film'],
  ['mp4', 'i-lucide-film'],
  ['avi', 'i-lucide-film'],
  ['mov', 'i-lucide-film'],
  ['wmv', 'i-lucide-film'],
  ['webm', 'i-lucide-film'],
  ['flv', 'i-lucide-film'],
  ['m4v', 'i-lucide-film'],
  ['mpg', 'i-lucide-film'],
  ['mpeg', 'i-lucide-film'],
  ['vob', 'i-lucide-film'],
  ['jpg', 'i-lucide-image'],
  ['jpeg', 'i-lucide-image'],
  ['png', 'i-lucide-image'],
  ['gif', 'i-lucide-image'],
  ['webp', 'i-lucide-image'],
  ['bmp', 'i-lucide-image'],
  ['heic', 'i-lucide-image'],
  ['tiff', 'i-lucide-image'],
  ['svg', 'i-lucide-image'],
  ['mp3', 'i-lucide-music'],
  ['flac', 'i-lucide-music'],
  ['wav', 'i-lucide-music'],
  ['m4a', 'i-lucide-music'],
  ['aac', 'i-lucide-music'],
  ['ogg', 'i-lucide-music'],
  ['opus', 'i-lucide-music'],
  ['wma', 'i-lucide-music'],
  ['zip', 'i-lucide-archive'],
  ['rar', 'i-lucide-archive'],
  ['7z', 'i-lucide-archive'],
  ['tar', 'i-lucide-archive'],
  ['gz', 'i-lucide-archive'],
  ['bz2', 'i-lucide-archive'],
  ['xz', 'i-lucide-archive'],
  ['iso', 'i-lucide-archive'],
  ['srt', 'i-lucide-captions'],
  ['sub', 'i-lucide-captions'],
  ['ass', 'i-lucide-captions'],
  ['ssa', 'i-lucide-captions'],
  ['vtt', 'i-lucide-captions'],
  ['pdf', 'i-lucide-book-open'],
  ['epub', 'i-lucide-book-open'],
  ['mobi', 'i-lucide-book-open'],
  ['txt', 'i-lucide-book-open'],
  ['nfo', 'i-lucide-book-open'],
  ['md', 'i-lucide-book-open'],
  ['doc', 'i-lucide-book-open'],
  ['docx', 'i-lucide-book-open'],
  ['rtf', 'i-lucide-book-open'],
])

export const fileIconFor = (path: string) => {
  const name = path.split(segmentPattern).pop() ?? ''
  const extensionStart = name.lastIndexOf('.')
  if (extensionStart < 0) return genericFileIcon
  return iconByExtension.get(name.slice(extensionStart + 1).toLowerCase()) ?? genericFileIcon
}

// "No folder" layout: remove the single root directory shared by every
// file, the way qBittorrent strips it before placing files in the save
// location. Torrents without a common root are left untouched.
export const stripRootFolder = (files: TorrentTreeFileInput[]) => {
  if (files.length < 2) return files

  const segments = files.map(file => file.path.split(segmentPattern).filter(Boolean))
  const root = segments[0]![0]
  if (!root || !segments.every(segment => segment[0] === root)) return files

  return files.map((file, index) => {
    const stripped = segments[index]!.slice(1).join('/')
    return stripped ? { ...file, path: stripped } : file
  })
}
