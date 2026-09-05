import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { tauriQbittorrentAdapter } from '~/adapters/qbittorrent'
import TorrentDetailPanel from '~/components/TorrentDetailPanel.vue'
import type { ConnectionSnapshot } from '~/types/torrent'

const torrent = {
  id: 'abc123',
  name: 'BigBuckBunny_124',
  status: 'downloading' as const,
  progress: 62.5,
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
  it('does not restart polling when an in-flight request finishes after unmount', async () => {
    seedConnectedLibrary()
    let finishFiles!: (files: []) => void
    const fetchFiles = vi.spyOn(tauriQbittorrentAdapter, 'fetchTorrentFiles')
      .mockImplementation(() => new Promise(resolve => { finishFiles = resolve }))
    const wrapper = await mountSuspended(TorrentDetailPanel, { props: { torrentId: 'abc123' } })
    try {
      expect(fetchFiles).toHaveBeenCalledOnce()
      wrapper.unmount()
      vi.useFakeTimers()
      finishFiles([])
      await flushPromises()
      await vi.advanceTimersByTimeAsync(3_000)
      expect(fetchFiles).toHaveBeenCalledOnce()
    }
    finally {
      vi.useRealTimers()
      fetchFiles.mockRestore()
    }
  })

  it('renders the bound torrent with its facts and tags', async () => {
    seedConnectedLibrary()
    const wrapper = await mountSuspended(TorrentDetailPanel, { props: { torrentId: 'abc123' } })
    await flushPromises()

    expect(wrapper.text()).toContain('BigBuckBunny_124')
    expect(wrapper.text()).toContain('Downloading')
    expect(wrapper.text()).toContain('62.5%')
    expect(wrapper.text()).not.toContain('6250%')
    expect(wrapper.text()).toContain('1.5 KB remaining')
    expect(wrapper.text()).toContain('Location')
    expect(wrapper.text()).toContain('C:/Downloads')
    expect(wrapper.text()).toContain('Trackers')
    expect(wrapper.text()).toContain('Files')

    const tabs = wrapper.findAll('[role="tab"]')
    await tabs.find(tab => tab.text().includes('Files'))!.trigger('mousedown', { button: 0, ctrlKey: false })
    await flushPromises()
    expect(wrapper.text()).toContain('The file list is unavailable right now.')
    expect(wrapper.text()).not.toContain('Total size')

    await tabs.find(tab => tab.text().includes('Trackers'))!.trigger('mousedown', { button: 0, ctrlKey: false })
    await flushPromises()
    expect(wrapper.text()).toContain('The tracker list is unavailable right now.')

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
