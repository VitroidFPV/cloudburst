import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import AddTorrentModal from '~/components/AddTorrentModal.vue'
import type { AddTorrentFile, AddTorrentsInput, MetadataFetch, TorrentMetadata } from '~/types/torrent'

const mountedWrappers: VueWrapper[] = []

interface ModalProps {
  categories?: string[]
  defaultSavePath?: string
  canBrowse?: boolean
  pending?: boolean
  parseMetadata?: (files: AddTorrentFile[]) => Promise<TorrentMetadata[] | null>
  fetchMetadata?: (source: string) => Promise<MetadataFetch | null>
}

const mountModal = async (props: ModalProps = {}) => {
  const wrapper = await mountSuspended(AddTorrentModal, {
    props: {
      categories: ['Linux', 'Media'],
      defaultSavePath: 'C:/Downloads',
      canBrowse: false,
      pending: false,
      parseMetadata: async () => null,
      fetchMetadata: async () => ({ status: 'pending' }),
      ...props,
    },
  })
  mountedWrappers.push(wrapper)
  return wrapper
}

afterEach(() => {
  mountedWrappers.forEach(wrapper => wrapper.unmount())
  mountedWrappers.length = 0
})

const findInModal = <T extends HTMLElement>(selector: string) =>
  Array.from(document.body.querySelectorAll<T>(selector))

const setTextareaValue = (textarea: HTMLTextAreaElement, value: string) => {
  textarea.value = value
  textarea.dispatchEvent(new Event('input', { bubbles: true }))
}

const clickButtonWithLabel = async (label: string) => {
  const button = findInModal<HTMLButtonElement>('button')
    .find(candidate => candidate.textContent?.trim() === label)
  expect(button, `expected a "${label}" button`).toBeDefined()
  button!.click()
  await flushPromises()
}

const emittedInput = (wrapper: VueWrapper) =>
  (wrapper.emitted('add') as Array<[AddTorrentsInput]>).at(-1)![0]

describe('AddTorrentModal', () => {
  beforeEach(() => localStorage.clear())

  it('walks a two-step flow for a single magnet and adds it unmodified', async () => {
    const wrapper = await mountModal()
    ;(wrapper.vm as unknown as { openWith: (options?: object) => void }).openWith()
    await flushPromises()

    expect(document.body.textContent).toContain('Add torrents')

    const textarea = findInModal<HTMLTextAreaElement>('textarea')[0]!
    setTextareaValue(textarea, 'magnet:?xt=urn:btih:abc')
    await flushPromises()

    await clickButtonWithLabel('Continue')
    expect(document.body.textContent).toContain('Review torrent')

    await clickButtonWithLabel('Add torrent')

    expect(emittedInput(wrapper)).toEqual({
      urls: ['magnet:?xt=urn:btih:abc'],
      files: [],
      category: undefined,
      savePath: undefined,
      contentLayout: 'original',
      filePriorities: undefined,
    })
  })

  it('adds a batch of sources in one request without a file tree', async () => {
    const wrapper = await mountModal()
    ;(wrapper.vm as unknown as { openWith: (options?: object) => void }).openWith()
    await flushPromises()

    const textarea = findInModal<HTMLTextAreaElement>('textarea')[0]!
    setTextareaValue(textarea, ' magnet:?xt=urn:btih:abc \nhttps://example.test/debian.torrent')
    await flushPromises()

    await clickButtonWithLabel('Continue')
    expect(document.body.textContent).not.toContain('Fetching metadata')

    await clickButtonWithLabel('Add 2 torrents')

    expect(emittedInput(wrapper)).toEqual({
      urls: ['magnet:?xt=urn:btih:abc', 'https://example.test/debian.torrent'],
      files: [],
      category: undefined,
      savePath: undefined,
      contentLayout: 'original',
      filePriorities: undefined,
    })
  })

  it('chooses the folder layout from the large radio options', async () => {
    const wrapper = await mountModal()
    ;(wrapper.vm as unknown as { openWith: (options?: { urls?: string[] }) => void })
      .openWith({ urls: ['magnet:?xt=urn:btih:abc', 'magnet:?xt=urn:btih:def'] })
    await flushPromises()

    await clickButtonWithLabel('Continue')

    const layoutGroup = findInModal<HTMLElement>('[role="radiogroup"][aria-label="Folder layout"]')[0]
    expect(layoutGroup).toBeDefined()
    const noFolderLabel = Array.from(layoutGroup!.querySelectorAll('label'))
      .find(option => option.textContent?.trim() === 'No Folder')
    expect(noFolderLabel).toBeDefined()
    const noFolderOption = noFolderLabel!.querySelector<HTMLButtonElement>('[role="radio"]')
    expect(noFolderOption).toBeDefined()
    noFolderOption!.click()
    await flushPromises()

    await clickButtonWithLabel('Add 2 torrents')
    expect(emittedInput(wrapper).contentLayout).toBe('noSubfolder')
  })

  it('remembers a submitted save location for future torrents when opted in', async () => {
    const wrapper = await mountModal()
    const sources = ['magnet:?xt=urn:btih:abc', 'magnet:?xt=urn:btih:def']
    ;(wrapper.vm as unknown as { openWith: (options?: { urls?: string[] }) => void }).openWith({ urls: sources })
    await flushPromises()
    await clickButtonWithLabel('Continue')

    const saveLocation = findInModal<HTMLInputElement>('[aria-label="Save location"]')[0]!
    saveLocation.value = 'D:/Torrents'
    saveLocation.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()

    const remember = findInModal<HTMLButtonElement>('[role="switch"]')[0]!
    remember.click()
    await flushPromises()
    await clickButtonWithLabel('Add 2 torrents')

    expect(localStorage.getItem('cloudburst:last-save-path')).toBe('D:/Torrents')
    expect(emittedInput(wrapper).savePath).toBe('D:/Torrents')

    ;(wrapper.vm as unknown as { openWith: (options?: { urls?: string[] }) => void }).openWith({ urls: sources })
    await flushPromises()
    await clickButtonWithLabel('Continue')

    expect(findInModal<HTMLInputElement>('[aria-label="Save location"]')[0]!.value).toBe('D:/Torrents')
    expect(findInModal<HTMLButtonElement>('[role="switch"]')[0]!.getAttribute('data-state')).toBe('checked')
  })

  it('shows the file tree for a magnet with metadata and applies partial selection', async () => {
    const fetchMetadata = async (): Promise<MetadataFetch> => ({
      status: 'ready',
      metadata: {
        hash: 'v2hash',
        name: 'Show.S01',
        files: [
          { path: 'Show.S01/ep1.mkv', length: 1000 },
          { path: 'Show.S01/ep2.mkv', length: 2000 },
        ],
      },
    })
    const wrapper = await mountModal({ fetchMetadata })
    ;(wrapper.vm as unknown as { openWith: (options?: { urls?: string[] }) => void })
      .openWith({ urls: ['magnet:?xt=urn:btih:abc'] })
    await flushPromises()

    await clickButtonWithLabel('Continue')
    await flushPromises()

    expect(document.body.textContent).toContain('Show.S01')
    expect(document.body.textContent).toContain('ep2.mkv')

    // ep2 starts at Normal; clicking its active rating step clears it to skip.
    const ep2Rating = findInModal<HTMLButtonElement>('[role="radiogroup"][aria-label="Priority for ep2.mkv"] button[role="radio"]')[0]!
    ep2Rating.click()
    await flushPromises()

    await clickButtonWithLabel('Add torrent')

    expect(emittedInput(wrapper).urls).toEqual(['magnet:?xt=urn:btih:abc'])
    expect(emittedInput(wrapper).filePriorities).toEqual([1, 0])
  })

  it('preserves an existing torrent root when Folder is selected', async () => {
    const fetchMetadata = async (): Promise<MetadataFetch> => ({
      status: 'ready',
      metadata: {
        hash: 'v2hash',
        name: 'Show.S01',
        files: [
          { path: 'Show.S01/ep1.mkv', length: 1000 },
          { path: 'Show.S01/ep2.mkv', length: 2000 },
        ],
      },
    })
    const wrapper = await mountModal({ fetchMetadata })
    ;(wrapper.vm as unknown as { openWith: (options?: { urls?: string[] }) => void })
      .openWith({ urls: ['magnet:?xt=urn:btih:abc'] })
    await flushPromises()

    await clickButtonWithLabel('Continue')
    await flushPromises()

    const folderLabel = Array.from(document.body.querySelectorAll('label'))
      .find(option => option.textContent?.trim() === 'Folder')
    folderLabel!.querySelector<HTMLButtonElement>('[role="radio"]')!.click()
    await flushPromises()
    await clickButtonWithLabel('Add torrent')

    expect(emittedInput(wrapper).contentLayout).toBe('original')
  })

  it('adds a parsed single file by hash so priorities can apply', async () => {
    const parseMetadata = async (): Promise<TorrentMetadata[] | null> => [{
      hash: 'v2hash',
      name: 'Show.S01',
      files: [{ path: 'Show.S01/ep1.mkv', length: 1000 }],
    }]
    const wrapper = await mountModal({ parseMetadata })
    const file = new File([new Uint8Array([100])], 'show.torrent', { type: 'application/x-bittorrent' })
    ;(wrapper.vm as unknown as { openWith: (options?: { files?: File[] }) => void })
      .openWith({ files: [file] })
    await flushPromises()

    await clickButtonWithLabel('Continue')
    await flushPromises()

    expect(document.body.textContent).toContain('ep1.mkv')

    await clickButtonWithLabel('Add torrent')

    expect(emittedInput(wrapper).urls).toEqual(['v2hash'])
    expect(emittedInput(wrapper).files).toEqual([])
    expect(emittedInput(wrapper).filePriorities).toBeUndefined()
  })
})
