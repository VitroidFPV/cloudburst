import { mountSuspended } from '@nuxt/test-utils/runtime'
import { beforeEach, describe, expect, it } from 'vitest'
import TorrentTable from '~/components/TorrentTable.vue'

const storageKeys = [
  'cloudburst:torrent-column-sizing',
  'cloudburst:torrent-column-visibility',
  'cloudburst:torrent-column-sorting',
]

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
})
