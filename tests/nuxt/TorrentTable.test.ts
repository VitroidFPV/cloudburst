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

// Overlays teleport to document.body and outlive their test wrappers, so
// leftover menus and dialogs must be dropped before asserting on new ones.
const dropLeftoverOverlays = () => {
  document.body
    .querySelectorAll('[role="menu"], [role="dialog"], [role="listbox"]')
    .forEach(overlay => overlay.remove())
}

describe('TorrentTable', () => {
  beforeEach(() => {
    localStorage.clear()
    dropLeftoverOverlays()
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

    expect(wrapper.find('[aria-label="Choose visible columns"]').exists()).toBe(true)
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

  it('confirms before removing selected torrents', async () => {
    const wrapper = await mountSuspended(TorrentTable, { props: { torrents: [torrent] } })
    await wrapper.get('[aria-label="Select all torrents"]').trigger('click')

    await wrapper.get('[aria-label="Remove selected torrents"]').trigger('click')
    await flushPromises()
    const modal = document.body

    expect(modal.textContent).toContain('Remove torrent')
    expect(modal.textContent).toContain('Also remove downloaded files')
    expect(wrapper.emitted('remove-torrents')).toBeUndefined()

    const removeButtons = Array.from(modal.querySelectorAll('button'))
      .filter(button => button.textContent?.trim() === 'Remove')

    expect(removeButtons).toHaveLength(1)
    removeButtons[0]!.click()
    await flushPromises()
    expect(wrapper.emitted('remove-torrents')).toEqual([[['torrent-1'], false]])

    await wrapper.get('[aria-label="Remove selected torrents"]').trigger('click')
    await flushPromises()
    const removeFiles = Array.from(document.body.querySelectorAll<HTMLButtonElement>('[role="switch"]')).at(-1)
    expect(removeFiles).toBeDefined()
    removeFiles!.click()
    await flushPromises()
    Array.from(document.body.querySelectorAll('button'))
      .find(button => button.textContent?.trim() === 'Remove')!.click()
    await flushPromises()

    expect(wrapper.emitted('remove-torrents')).toEqual([
      [['torrent-1'], false],
      [['torrent-1'], true],
    ])
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

  it('selects torrents handed over by an external action', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [torrent], autoSelectIds: [] },
    })
    expect(wrapper.text()).not.toContain('1 selected')

    await wrapper.setProps({ autoSelectIds: ['torrent-1'] })

    expect(wrapper.text()).toContain('1 selected')
  })

  it('searches torrent names, categories, tags, and statuses', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: {
        torrents: [
          torrent,
          { ...torrent, id: 'torrent-2', name: 'Big Buck Bunny', category: 'Movies', tags: ['feature'], status: 'paused' },
          { ...torrent, id: 'torrent-3', name: 'Fedora ISO', category: 'Linux', tags: ['iso'], status: 'seeding' },
        ],
      },
    })
    const search = wrapper.get('[aria-label="Search torrents"]')

    await search.setValue('movies')
    await flushPromises()
    expect(wrapper.text()).toContain('1 of 3')

    await search.setValue('linux')
    await flushPromises()
    expect(wrapper.text()).toContain('2 of 3')
  })

  it('keeps paused torrent names readable while leaving their status icon subdued', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [{ ...torrent, status: 'paused' }] },
    })
    const torrentName = wrapper.findAll('p').find(node => node.text() === torrent.name)

    expect(torrentName?.classes()).toContain('text-(--cloudburst-paused-name)')
    expect(torrentName?.classes()).not.toContain('text-dimmed')
  })

  it('exposes explicit pointer and keyboard detail actions', async () => {
    const wrapper = await mountSuspended(TorrentTable, { props: { torrents: [torrent] } })

    await wrapper.get('[aria-label="Open details for Debian ISO"]').trigger('click')
    expect(wrapper.emitted('toggle-details')).toEqual([['torrent-1']])

    await wrapper.get('tbody tr').trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('toggle-details')).toEqual([['torrent-1'], ['torrent-1']])
  })

  it('toggles details from the chevron and labels it for the open state', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [torrent], openTorrentId: null },
    })

    const chevron = () => wrapper.get('[aria-label="Open details for Debian ISO"]')
    await chevron().trigger('click')
    expect(wrapper.emitted('toggle-details')).toEqual([['torrent-1']])

    await wrapper.setProps({ openTorrentId: 'torrent-1' })
    const closeChevron = wrapper.get('[aria-label="Close details for Debian ISO"]')
    expect(closeChevron.attributes('aria-expanded')).toBe('true')

    await closeChevron.trigger('click')
    expect(wrapper.emitted('toggle-details')).toEqual([['torrent-1'], ['torrent-1']])
  })

  it('offers a closing context menu action while the selection is open', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [torrent], openTorrentId: 'torrent-1' },
    })

    await wrapper.get('[aria-label="Select all torrents"]').trigger('click')
    await wrapper.get('.torrent-table').trigger('contextmenu', { button: 2, clientX: 40, clientY: 40 })
    await flushPromises()
    const menus = document.body.querySelectorAll('[role="menu"]')
    const menu = menus[menus.length - 1]

    expect(menu?.textContent).toContain('Close details')
    const closeItem = Array.from(menu!.querySelectorAll('[role="menuitem"]'))
      .find(item => item.textContent?.includes('Close details'))!
    closeItem.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPromises()

    expect(wrapper.emitted('toggle-details')).toEqual([['torrent-1']])
    await wrapper.unmount()
  })

  it('closes open details with Escape before clearing the selection', async () => {
    dropLeftoverOverlays()
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [torrent], openTorrentId: 'torrent-1' },
    })

    await wrapper.get('[aria-label="Select all torrents"]').trigger('click')
    expect(wrapper.text()).toContain('1 selected')

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(wrapper.emitted('toggle-details')).toEqual([['torrent-1']])

    await wrapper.setProps({ openTorrentId: null })
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await flushPromises()

    expect(wrapper.text()).not.toContain('1 selected')
    expect(wrapper.emitted('toggle-details')).toHaveLength(1)
    await wrapper.unmount()
  })

  it('leaves dialogs to handle Escape and keeps the selection', async () => {
    const wrapper = await mountSuspended(TorrentTable, {
      props: { torrents: [torrent], openTorrentId: null },
    })
    await wrapper.get('[aria-label="Select all torrents"]').trigger('click')

    const dialog = document.createElement('div')
    dialog.setAttribute('role', 'dialog')
    document.body.appendChild(dialog)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    dialog.remove()

    expect(wrapper.emitted('toggle-details')).toBeUndefined()
    expect(wrapper.text()).toContain('1 selected')
  })

})
