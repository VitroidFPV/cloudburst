import { useColorMode } from '#imports'
import { mountSuspended } from '@nuxt/test-utils/runtime'
import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import AppSettingsModal from '~/components/AppSettingsModal.vue'

const mountedWrappers: VueWrapper[] = []

const mountModal = async (canConfigureMagnets = false) => {
  const wrapper = await mountSuspended(AppSettingsModal, { props: { open: true, canConfigureMagnets } })
  mountedWrappers.push(wrapper)
  await flushPromises()
  return wrapper
}

afterEach(() => {
  mountedWrappers.forEach(wrapper => wrapper.unmount())
  mountedWrappers.length = 0
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})

const bodyText = () => document.body.textContent ?? ''

const findRadioOption = (label: string) =>
  Array.from(document.body.querySelectorAll('label'))
    .find(element => element.textContent?.trim() === label)

const clickRadio = async (label: string) => {
  const option = findRadioOption(label)
  expect(option, `expected a "${label}" radio option`).toBeDefined()
  const radio = option!.querySelector<HTMLButtonElement>('[role="radio"]')
  expect(radio, `expected a radio input for "${label}"`).toBeDefined()
  radio!.click()
  await flushPromises()
}

describe('AppSettingsModal', () => {
  beforeEach(() => {
    localStorage.clear()
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  })

  it('offers window material, color mode, refresh cadence, and placeholder settings', async () => {
    await mountModal()

    expect(bodyText()).toContain('Window material')
    expect(bodyText()).toContain('Color mode')
    expect(bodyText()).toContain('Refresh cadence')
    expect(bodyText()).toContain('Placeholder torrents')
    expect(findRadioOption('Mica')).toBeDefined()
    expect(findRadioOption('Dark')).toBeDefined()
    expect(findRadioOption('Slow')).toBeDefined()
  })

  it('applies and persists a color mode choice', async () => {
    await mountModal()

    await clickRadio('Dark')

    expect(useColorMode().preference).toBe('dark')
  })

  it('applies and persists a refresh cadence choice', async () => {
    await mountModal()

    await clickRadio('Slow')

    expect(localStorage.getItem('cloudburst:refresh-cadence')).toBe('slow')
  })

  it('enables the placeholder library from the switch', async () => {
    await mountModal()

    const placeholderSwitch = document.body.querySelector<HTMLButtonElement>('[role="switch"][aria-label="Placeholder torrents"]')
    expect(placeholderSwitch).toBeDefined()
    expect(placeholderSwitch!.getAttribute('aria-checked')).toBe('false')
    expect(placeholderSwitch!.hasAttribute('disabled')).toBe(false)

    placeholderSwitch!.click()
    await flushPromises()

    expect(localStorage.getItem('cloudburst:placeholder-enabled')).toBe('true')
  })

  it('keeps magnet setup accessible from Windows settings', async () => {
    const wrapper = await mountModal(true)
    const setup = document.body.querySelector<HTMLButtonElement>('[aria-label="Set up magnet links"]')

    expect(setup).toBeDefined()
    setup!.click()
    await flushPromises()

    expect(wrapper.emitted('magnet-settings')).toEqual([[]])
  })
})
