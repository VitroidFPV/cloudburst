import { describe, expect, it } from 'vitest'
import { resolveTorrentSelection, shouldSelectAllTorrents } from '../../app/utils/torrent-selection'

const orderedIds = ['torrent-1', 'torrent-2', 'torrent-3', 'torrent-4', 'torrent-5']

describe('torrent row selection', () => {
  it('selects all only when the current selection is empty', () => {
    expect(shouldSelectAllTorrents(false, false)).toBe(true)
    expect(shouldSelectAllTorrents(true, false)).toBe(false)
    expect(shouldSelectAllTorrents(false, true)).toBe(false)
  })

  it('supports plain, Ctrl/Cmd, Shift, and additive range selection', () => {
    let result = resolveTorrentSelection({
      orderedIds,
      targetId: 'torrent-1',
      selected: {},
      additive: false,
      range: false,
    })
    expect(result).toEqual({ selected: { 'torrent-1': true }, anchorId: 'torrent-1' })

    result = resolveTorrentSelection({
      orderedIds,
      targetId: 'torrent-3',
      selected: result.selected,
      anchorId: result.anchorId,
      additive: true,
      range: false,
    })
    expect(result.selected).toEqual({ 'torrent-1': true, 'torrent-3': true })

    result = resolveTorrentSelection({
      orderedIds,
      targetId: 'torrent-5',
      selected: result.selected,
      anchorId: result.anchorId,
      additive: false,
      range: true,
    })
    expect(result).toEqual({
      selected: { 'torrent-3': true, 'torrent-4': true, 'torrent-5': true },
      anchorId: 'torrent-3',
    })

    result = resolveTorrentSelection({
      orderedIds,
      targetId: 'torrent-1',
      selected: result.selected,
      anchorId: result.anchorId,
      additive: true,
      range: true,
    })
    expect(Object.keys(result.selected).sort()).toEqual(orderedIds)
  })

  it('toggles an individual row without affecting the rest of the selection', () => {
    const result = resolveTorrentSelection({
      orderedIds,
      targetId: 'torrent-3',
      selected: { 'torrent-1': true, 'torrent-3': true },
      anchorId: 'torrent-1',
      additive: true,
      range: false,
    })

    expect(result).toEqual({ selected: { 'torrent-1': true }, anchorId: 'torrent-3' })
  })
})
