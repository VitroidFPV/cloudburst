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

describe('TorrentFileTree', () => {
  it('starts with everything selected and reports the change payload', async () => {
    const wrapper = await mountSuspended(TorrentFileTree, { props: { files } })
    await flushPromises()

    expect(wrapper.text()).toContain('2.9 KB')

    await wrapper.get('[role="checkbox"][aria-label="Include ep2.mkv"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('1005.0 B of 2.9 KB')

    const change = wrapper.emitted('change') as Array<Array<ChangePayload>>
    expect(change.at(-1)![0]).toEqual({
      priorities: [1, 0, 1],
      selectedSize: 1005,
      allSelected: false,
    })
  })

  it('propagates a folder toggle to every contained file', async () => {
    const wrapper = await mountSuspended(TorrentFileTree, { props: { files } })
    await flushPromises()

    await wrapper.get('[role="checkbox"][aria-label="Include Show"]').trigger('click')
    await flushPromises()

    const change = wrapper.emitted('change') as Array<Array<ChangePayload>>
    expect(change.at(-1)![0]).toEqual({
      priorities: [0, 0, 1],
      selectedSize: 5,
      allSelected: false,
    })

    await wrapper.get('[role="checkbox"][aria-label="Include ep2.mkv"]').trigger('click')
    await flushPromises()
    const latest = (wrapper.emitted('change') as Array<Array<ChangePayload>>).at(-1)![0]
    expect(latest!.priorities).toEqual([0, 1, 1])
  })

  it('keeps the torrent layout honest in the tree shape', async () => {
    const rootedFiles: TorrentMetadataFile[] = [
      { path: 'Show.S01/ep1.mkv', length: 1000 },
      { path: 'Show.S01/ep2.mkv', length: 2000 },
      { path: 'Show.S01/readme.txt', length: 5 },
    ]

    const subfolder = await mountSuspended(TorrentFileTree, {
      props: { files: rootedFiles, layout: 'subfolder', folderName: 'New Folder' },
    })
    await flushPromises()
    expect(subfolder.text()).toContain('New Folder')
    expect(subfolder.text()).toContain('Show.S01')

    const noSubfolder = await mountSuspended(TorrentFileTree, {
      props: { files: rootedFiles, layout: 'noSubfolder' },
    })
    await flushPromises()
    expect(noSubfolder.text()).toContain('ep1.mkv')
    expect(noSubfolder.find('[role="checkbox"][aria-label="Include Show.S01"]').exists()).toBe(false)
    expect(noSubfolder.find('[role="checkbox"][aria-label="Include readme.txt"]').exists()).toBe(true)
  })
})