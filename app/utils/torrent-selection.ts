export interface TorrentSelectionInput {
  orderedIds: string[]
  targetId: string
  selected: Record<string, boolean>
  anchorId?: string
  additive: boolean
  range: boolean
}

export interface TorrentSelectionResult {
  selected: Record<string, boolean>
  anchorId: string
}

export const shouldSelectAllTorrents = (someSelected: boolean, allSelected: boolean) => !someSelected && !allSelected

export const resolveTorrentSelection = (input: TorrentSelectionInput): TorrentSelectionResult => {
  if (input.range && input.anchorId) {
    const anchorIndex = input.orderedIds.indexOf(input.anchorId)
    const targetIndex = input.orderedIds.indexOf(input.targetId)

    if (anchorIndex >= 0 && targetIndex >= 0) {
      const [rangeStart, rangeEnd] = anchorIndex < targetIndex
        ? [anchorIndex, targetIndex]
        : [targetIndex, anchorIndex]
      const selected = input.additive ? { ...input.selected } : {}
      input.orderedIds.slice(rangeStart, rangeEnd + 1).forEach((id) => {
        selected[id] = true
      })
      return { selected, anchorId: input.anchorId }
    }
  }

  if (input.additive) {
    const selected = input.selected[input.targetId]
      ? Object.fromEntries(Object.entries(input.selected).filter(([id]) => id !== input.targetId))
      : { ...input.selected, [input.targetId]: true }
    return { selected, anchorId: input.targetId }
  }

  return { selected: { [input.targetId]: true }, anchorId: input.targetId }
}
