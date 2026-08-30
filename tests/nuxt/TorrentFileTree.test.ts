import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TorrentFileTree from '~/components/TorrentFileTree.vue'
import type { TorrentMetadataFile } from '~/types/torrent'

const files: TorrentMetadataFile[] = [
  { path: 'Show/ep1.mkv', length: 1000 },
  { path: 'Show/ep2.mkv', length: 2000 },
  { path: 'readme.txt', length: 5 },
]

type ChangePayload = { priorities: number[], selectedSize: number, allSelected: boolean }

const latestChange = (wrapper: Awaited<ReturnType<typeof mountSuspended<typeof TorrentFileTree>>>) =>
  (wrapper.emitted('change') as Array<Array<ChangePayload>>).at(-1)![0]!

// A rating renders as a radiogroup of radio buttons; clicking step N sets
// that rating, and clicking the currently active step again clears it.
const clickRatingStep = async (wrapper: Awaited<ReturnType<typeof mountSuspended<typeof TorrentFileTree>>>, name: string, step: number) => {
  const group = wrapper.get(`[role="radiogroup"][aria-label="Priority for ${name}"]`)
  const radios = group.findAll('button[role="radio"]')
  await radios[step - 1]!.trigger('click')
  await flushPromises()
}

describe('TorrentFileTree', () => {
  it('starts every file at normal priority and reports the change payload', async () => {
    const wrapper = await mountSuspended(TorrentFileTree, { props: { files } })
    await flushPromises()

    expect(wrapper.text()).toContain('2.9 KB')
    expect(wrapper.findAll('[role="radiogroup"]')).toHaveLength(3)
    expect(wrapper.find('[role="checkbox"][aria-label="Include Show"]').exists()).toBe(true)

    expect(latestChange(wrapper)).toEqual({
      priorities: [1, 1, 1],
      selectedSize: 3005,
      allSelected: true,
    })
  })

  it('maps rating steps onto qBittorrent priorities per file', async () => {
    const wrapper = await mountSuspended(TorrentFileTree, { props: { files } })
    await flushPromises()

    await clickRatingStep(wrapper, 'ep1.mkv', 1) // clears Normal
    await clickRatingStep(wrapper, 'readme.txt', 3)
    expect(latestChange(wrapper)).toEqual({
      priorities: [0, 1, 7],
      selectedSize: 2005,
      allSelected: false,
    })

    await clickRatingStep(wrapper, 'ep1.mkv', 2)
    expect(latestChange(wrapper)).toEqual({
      priorities: [6, 1, 7],
      selectedSize: 3005,
      allSelected: true,
    })
  })

  it('propagates a folder toggle to every contained file', async () => {
    const wrapper = await mountSuspended(TorrentFileTree, { props: { files } })
    await flushPromises()

    await wrapper.get('[role="checkbox"][aria-label="Include Show"]').trigger('click')
    await flushPromises()

    expect(latestChange(wrapper)).toEqual({
      priorities: [0, 0, 1],
      selectedSize: 5,
      allSelected: false,
    })

    await clickRatingStep(wrapper, 'ep2.mkv', 1)
    expect(latestChange(wrapper)).toEqual({
      priorities: [0, 1, 1],
      selectedSize: 2005,
      allSelected: false,
    })
  })

  it('seeds ratings from the instance, survives polls, and resets with the reset key', async () => {
    const wrapper = await mountSuspended(TorrentFileTree, {
      props: { files, priorities: [1, 6, 0], resetKey: 'abc123' },
    })
    await flushPromises()

    expect(latestChange(wrapper)).toEqual({
      priorities: [1, 6, 0],
      selectedSize: 3000,
      allSelected: false,
    })

    await wrapper.setProps({ files: files.map(file => ({ ...file })) })
    await flushPromises()
    expect(latestChange(wrapper)).toEqual({
      priorities: [1, 6, 0],
      selectedSize: 3000,
      allSelected: false,
    })

    await clickRatingStep(wrapper, 'ep2.mkv', 3)
    expect(latestChange(wrapper)).toEqual({
      priorities: [1, 7, 0],
      selectedSize: 3000,
      allSelected: false,
    })

    await wrapper.setProps({ resetKey: 'def456' })
    await flushPromises()
    expect(latestChange(wrapper)).toEqual({
      priorities: [1, 6, 0],
      selectedSize: 3000,
      allSelected: false,
    })
  })

  it('shows per-file progress for partially downloaded files', async () => {
    const partial = files.map((file, index) => ({ ...file, progress: index === 0 ? 0.25 : 1 }))
    const wrapper = await mountSuspended(TorrentFileTree, { props: { files: partial } })
    await flushPromises()

    expect(wrapper.text()).toContain('25%')
    expect(wrapper.text()).not.toContain('100%')
  })

  it('keeps the torrent layout honest in the tree shape', async () => {
    const rootedFiles: TorrentMetadataFile[] = [
      { path: 'Show.S01/ep1.mkv', length: 1000 },
      { path: 'Show.S01/ep2.mkv', length: 2000 },
      { path: 'Show.S01/readme.txt', length: 5 },
    ]

    const subfolder = await mountSuspended(TorrentFileTree, {
      props: {
        files: [
          { path: 'ep1.mkv', length: 1000 },
          { path: 'ep2.mkv', length: 2000 },
        ],
        layout: 'subfolder',
        folderName: 'New Folder',
      },
    })
    await flushPromises()
    expect(subfolder.text()).toContain('New Folder')
    expect(subfolder.text()).toContain('ep1.mkv')

    const noSubfolder = await mountSuspended(TorrentFileTree, {
      props: { files: rootedFiles, layout: 'noSubfolder' },
    })
    await flushPromises()
    expect(noSubfolder.text()).toContain('ep1.mkv')
    expect(noSubfolder.find('[role="checkbox"][aria-label="Include Show.S01"]').exists()).toBe(false)
    expect(noSubfolder.findAll('[role="radiogroup"]')).toHaveLength(3)
  })

  it('does not duplicate an existing root when folder layout is selected', async () => {
    const folderName = 'Jet Lag The Game - S19E02'
    const wrapper = await mountSuspended(TorrentFileTree, {
      props: {
        files: [
          { path: `${folderName}/episode.mkv`, length: 1000 },
          { path: `${folderName}/episode.vtt`, length: 50 },
        ],
        layout: 'subfolder',
        folderName,
      },
    })
    await flushPromises()

    expect(wrapper.findAll(`[role="checkbox"][aria-label="Include ${folderName}"]`)).toHaveLength(1)
  })
})
