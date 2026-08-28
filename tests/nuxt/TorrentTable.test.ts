import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import TorrentTable from '~/components/TorrentTable.vue'
import type { Torrent } from '~/types/torrent'

const storageKeys = [
  'cloudburst:torrent-column-sizing',
  'cloudburst:torrent-column-visibility',
  'cloudburst:torrent-column-sorting',
]

const torrent: Torrent = {
  id: 'torrent-1',
  name: 'Debian ISO',
  status: 'downloading',
  progress: 62.5,
  size: 4096,
  downloaded: 2560,
  downSpeed: 1024,
  upSpeed: 128,
  etaSeconds: 90,
  ratio: 0.5,
  seeds: 12,
  peers: 3,
  category: 'Linux',
  tags: ['iso'],
  addedOn: 1_700_000_000,
  savePath: 'C:/Downloads',
}

describe('TorrentTable', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('renders page-owned actions, notices, and empty content through its interface', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [] },
      slots: {
        actions: '<span>Connection action</span>',
        notice: '<p>Stale library</p>',
        empty: '<p>No matching torrents</p>',
      },
    })

    expect(wrapper.text()).toContain('Torrents')
    expect(wrapper.text()).toContain('Columns')
    expect(wrapper.text()).toContain('Connection action')
    expect(wrapper.text()).toContain('Stale library')
    expect(wrapper.text()).toContain('No matching torrents')
  })

  it('removes corrupt persisted table preferences', async () => {
    storageKeys.forEach(key => localStorage.setItem(key, '{invalid'))

    await mountSuspended(TorrentTable, { props: { torrents: [] } })

    storageKeys.forEach(key => expect(localStorage.getItem(key)).toBeNull())
  })

  it('uses one selected-torrent action interface for table controls', async () => {
    const wrapper = await mountSuspended(TorrentTable, { props: { torrents: [torrent] } })

    await wrapper.get('[aria-label="Select all torrents"]').trigger('click')
    expect(wrapper.text()).toContain('1 selected')

    await wrapper.get('[aria-label="Stop selected torrents"]').trigger('click')
    expect(wrapper.emitted('set-paused')).toEqual([[['torrent-1'], true]])

    await wrapper.get('.torrent-table').trigger('contextmenu', { button: 2, clientX: 40, clientY: 40 })
    await flushPromises()
    const menu = document.body.querySelector('[role="menu"]')

    expect(menu?.textContent).toContain('Start')
    expect(menu?.textContent).toContain('Stop')
    expect(menu?.textContent).not.toContain('Debian ISO')
  })

  it('selects and deselects every row atomically from the header checkbox', async () => {
    const torrents = Array.from({ length: 3 }, (_, index) => ({
      ...torrent,
      id: `torrent-${index + 1}`,
      name: `Torrent ${index + 1}`,
    }))
    const wrapper = await mountSuspended(TorrentTable, { props: { torrents } })
    const selectAll = wrapper.get('[aria-label="Select all torrents"]')

    await selectAll.trigger('click')
    expect(wrapper.text()).toContain('3 selected')

    await selectAll.trigger('click')
    expect(wrapper.text()).not.toContain('selected')
  })

})
