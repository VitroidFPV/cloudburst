import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import AddTorrentModal from '~/components/AddTorrentModal.vue'
import type { AddTorrentsInput } from '~/types/torrent'

const mountedWrappers: VueWrapper[] = []

const mountModal = async () => {
  const wrapper = await mountSuspended(AddTorrentModal, {
    props: {
      categories: ['Linux', 'Media'],
      defaultSavePath: 'C:/Downloads',
      canBrowse: false,
      pending: false,
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

const setInputValue = (input: HTMLInputElement, value: string) => {
  input.value = value
  input.dispatchEvent(new Event('input', { bubbles: true }))
}

const clickButtonWithLabel = async (label: string) => {
  const button = findInModal<HTMLButtonElement>('button')
    .find(candidate => candidate.textContent?.trim() === label)
  expect(button, `expected a "${label}" button`).toBeDefined()
  button!.click()
  await flushPromises()
}

describe('AddTorrentModal', () => {
  it('starts empty, accepts links, and emits a normalized add payload', async () => {
    const wrapper = await mountModal()
    ;(wrapper.vm as unknown as { openWith: (options?: object) => void }).openWith()
    await flushPromises()

    expect(document.body.textContent).toContain('Add torrents')
    expect(wrapper.emitted('add')).toBeUndefined()

    const textarea = findInModal<HTMLTextAreaElement>('textarea')[0]!
    textarea.value = ' magnet:?xt=urn:btih:abc \nhttps://example.test/debian.torrent'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()

    const category = findInModal<HTMLInputElement>('input[aria-label="Category"]')[0]!
    setInputValue(category, '  Linux  ')
    await flushPromises()

    await clickButtonWithLabel('Add 2 torrents')

    const emitted = wrapper.emitted('add') as Array<[AddTorrentsInput]>
    expect(emitted).toHaveLength(1)
    expect(emitted[0]![0]).toEqual({
      urls: ['magnet:?xt=urn:btih:abc', 'https://example.test/debian.torrent'],
      files: [],
      category: 'Linux',
      savePath: undefined,
      contentLayout: 'original',
    })
  })

  it('prefills a magnet from an external source and emits it on submit', async () => {
    const wrapper = await mountModal()
    ;(wrapper.vm as unknown as { openWith: (options?: { urls?: string[] }) => void })
      .openWith({ urls: ['magnet:?xt=urn:btih:abc'] })
    await flushPromises()

    const textarea = findInModal<HTMLTextAreaElement>('textarea')[0]!
    expect(textarea.value).toBe('magnet:?xt=urn:btih:abc')

    await clickButtonWithLabel('Add torrent')
    expect((wrapper.emitted('add') as Array<[AddTorrentsInput]>)[0]![0].urls).toEqual(['magnet:?xt=urn:btih:abc'])
  })
})
