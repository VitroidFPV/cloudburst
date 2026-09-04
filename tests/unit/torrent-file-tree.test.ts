import { describe, expect, it } from 'vitest'
import {
  buildFileTree,
  commonRootFolder,
  fileIconFor,
  isOpenableMediaPath,
  priorityForRating,
  priorityLabel,
  ratingForPriority,
  stripRootFolder,
} from '../../app/utils/torrent-file-tree'

describe('buildFileTree', () => {
  it('nests paths, sums folder sizes, and keeps file order', () => {
    const tree = buildFileTree([
      { path: 'Show/ep1.mkv', length: 1000 },
      { path: 'Show/ep2.mkv', length: 2000 },
      { path: 'Show/extras/notes.txt', length: 50 },
      { path: 'readme.txt', length: 5 },
    ])

    expect(tree).toHaveLength(2)
    const show = tree[0]!
    expect(show.isFolder).toBe(true)
    expect(show.size).toBe(3050)
    expect(show.progress).toBeNull()
    expect(show.children).toHaveLength(3)
    expect(show.children[0]!.fileIndex).toBe(0)
    const extras = show.children[2]!
    expect(extras.isFolder).toBe(true)
    expect(extras.size).toBe(50)
    expect(extras.children[0]!.fileIndex).toBe(2)

    const readme = tree[1]!
    expect(readme.isFolder).toBe(false)
    expect(readme.fileIndex).toBe(3)
    expect(readme.size).toBe(5)
  })

  it('carries per-file progress onto leaves only', () => {
    const tree = buildFileTree([
      { path: 'Show/ep1.mkv', length: 1000, progress: 0.25 },
      { path: 'Show/ep2.mkv', length: 2000 },
    ])

    expect(tree[0]!.progress).toBeNull()
    expect(tree[0]!.children[0]!.progress).toBe(0.25)
    expect(tree[0]!.children[1]!.progress).toBeNull()
  })

  it('maps priorities and rating steps onto each other', () => {
    expect(ratingForPriority(0)).toBe(0)
    expect(ratingForPriority(1)).toBe(1)
    expect(ratingForPriority(6)).toBe(2)
    expect(ratingForPriority(7)).toBe(3)
    expect(ratingForPriority(4)).toBe(1)

    expect(priorityForRating(0)).toBe(0)
    expect(priorityForRating(1)).toBe(1)
    expect(priorityForRating(2)).toBe(6)
    expect(priorityForRating(3)).toBe(7)

    expect(priorityLabel(0)).toBe('Skipped')
    expect(priorityLabel(1)).toBe('Normal')
    expect(priorityLabel(6)).toBe('High')
    expect(priorityLabel(7)).toBe('Maximum')
    expect(priorityLabel(4)).toBe('Normal')
  })

  it('treats windows separators as directory boundaries', () => {
    const tree = buildFileTree([{ path: 'ISO\\image.iso', length: 4096 }])

    const folder = tree[0]!
    expect(folder.name).toBe('ISO')
    expect(folder.isFolder).toBe(true)
    expect(folder.children[0]!.name).toBe('image.iso')
    expect(folder.children[0]!.fileIndex).toBe(0)
  })

  it('picks an icon from the file extension, case-insensitively', () => {
    expect(fileIconFor('movie.mkv')).toBe('i-lucide-film')
    expect(fileIconFor('SHOW/EPISODE.MP4')).toBe('i-lucide-film')
    expect(fileIconFor('poster.JPG')).toBe('i-lucide-image')
    expect(fileIconFor('soundtrack.flac')).toBe('i-lucide-music')
    expect(fileIconFor('packed.rar')).toBe('i-lucide-archive')
    expect(fileIconFor('disc.iso')).toBe('i-lucide-archive')
    expect(fileIconFor('subs/ep1.srt')).toBe('i-lucide-captions')
    expect(fileIconFor('release.nfo')).toBe('i-lucide-book-open')
    expect(fileIconFor('weird.unknown')).toBe('i-lucide-file')
    expect(fileIconFor('no-extension')).toBe('i-lucide-file')
  })

  it('allows only audio, video, and raster image files to open directly', () => {
    expect(isOpenableMediaPath('movie.MKV')).toBe(true)
    expect(isOpenableMediaPath('music/track.flac')).toBe(true)
    expect(isOpenableMediaPath('poster.webp')).toBe(true)
    expect(isOpenableMediaPath('vector.svg')).toBe(false)
    expect(isOpenableMediaPath('document.pdf')).toBe(false)
    expect(isOpenableMediaPath('installer.exe')).toBe(false)
  })

  it('strips a shared root folder and leaves multiple roots alone', () => {
    const stripped = stripRootFolder([
      { path: 'Show/ep1.mkv', length: 1 },
      { path: 'Show/extras/notes.txt', length: 2 },
    ])
    expect(stripped.map(file => file.path)).toEqual(['ep1.mkv', 'extras/notes.txt'])

    const untouched = stripRootFolder([
      { path: 'a/first.bin', length: 1 },
      { path: 'b/second.bin', length: 2 },
    ])
    expect(untouched.map(file => file.path)).toEqual(['a/first.bin', 'b/second.bin'])

    const single = stripRootFolder([{ path: 'only/file.bin', length: 1 }])
    expect(single[0]!.path).toBe('file.bin')
  })

  it('recognizes a root folder shared by every file', () => {
    expect(commonRootFolder([
      { path: 'Show/ep1.mkv', length: 1 },
      { path: 'Show/extras/notes.txt', length: 2 },
    ])).toBe('Show')
    expect(commonRootFolder([{ path: 'Show/only.mkv', length: 1 }])).toBe('Show')
    expect(commonRootFolder([
      { path: 'Show/ep1.mkv', length: 1 },
      { path: 'readme.txt', length: 2 },
    ])).toBeNull()
  })
})
