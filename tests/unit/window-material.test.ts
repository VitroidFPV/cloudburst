import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const css = readFileSync(new URL('../../app/assets/css/main.css', import.meta.url), 'utf8')

const ruleBody = (selector: string) => {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return css.match(new RegExp(`${escapedSelector}\\s*\\{([^}]+)\\}`))?.[1]
}

describe('full Mica surface', () => {
  it.each([
    'html.appearance-mica.dark #__nuxt',
    'html.appearance-mica:not(.dark) #__nuxt',
  ])('does not tint the native material for %s', (selector) => {
    expect(ruleBody(selector)).toMatch(/--ui-bg:\s*transparent;/)
  })
})
