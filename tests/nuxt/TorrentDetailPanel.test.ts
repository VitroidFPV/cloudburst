import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TorrentDetailPanel from '~/components/TorrentDetailPanel.vue'
import type { ConnectionSnapshot } from '~/types/torrent'

const torrent = {
  id: 'abc123',
  name: 'BigBuckBunny_124',
  status: 'downloading' as const,
  progress: 0.62,
  size: 4096,
  downloaded: 2560,
  downSpeed: 1024,
  upSpeed: 128,
  etaSeconds: 90,
  ratio: 0.5,
  seeds: 12,
  peers: 3,
  category: 'Movies',
  tags: ['hd'],
  addedOn: 1_700_000_000,
  savePath: 'C:/Downloads',
}

const seedConnectedLibrary = () => {
  const snapshot: ConnectionSnapshot = {
    endpoint: 'http://localhost:8080',
    version: '5.2.3',
    torrents: [torrent],
  }
  useState<ConnectionSnapshot['torrents']>('torrent-library', () => snapshot.torrents)
  useState('connection-status', () => 'connected' as const)
  useState('connection-error', () => '')
  useState('connection-endpoint', () => snapshot.endpoint)
  useState('connection-version', () => snapshot.version)
  useState('connection-stale', () => false)
}

describe('TorrentDetailPanel', () => {
  it('renders the bound torrent with its facts and tags', async () => {
    seedConnectedLibrary()
    const wrapper = await mountSuspended(TorrentDetailPanel, { props: { torrentId: 'abc123' } })
    await flushPromises()

    expect(wrapper.text()).toContain('BigBuckBunny_124')
    expect(wrapper.text()).toContain('Downloading')
    expect(wrapper.text()).toContain('Save location')
    expect(wrapper.text()).toContain('C:/Downloads')
    expect(wrapper.text()).toContain('Trackers')
    expect(wrapper.text()).toContain('Files')

    await wrapper.unmount()
  })

  it('closes itself when the bound torrent leaves the library', async () => {
    seedConnectedLibrary()
    const wrapper = await mountSuspended(TorrentDetailPanel, { props: { torrentId: 'abc123' } })
    await flushPromises()

    useState<ConnectionSnapshot['torrents']>('torrent-library', () => []).value = []
    await flushPromises()

    expect(wrapper.emitted('close')).toBeTruthy()
    await wrapper.unmount()
  })
})
