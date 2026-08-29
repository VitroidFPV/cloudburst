import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const css = readFileSync(new URL('../../app/assets/css/main.css', import.meta.url), 'utf8')

const ruleBody = (selector: string) => {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return Array.from(
    css.matchAll(new RegExp(`${escapedSelector}\\s*\\{([^}]+)\\}`, 'g')),
    match => match[1],
  ).join('\n')
}

describe('full Mica surface', () => {
  it.each([
    'html.appearance-mica.dark #__nuxt',
    'html.appearance-mica:not(.dark) #__nuxt',
  ])('does not tint the native material for %s', (selector) => {
    expect(ruleBody(selector)).toMatch(/--ui-bg:\s*transparent;/)
  })

  it('uses light overlays for raised dark surfaces', () => {
    const micaDark = ruleBody('html.appearance-mica.dark #__nuxt')

    expect(micaDark).toMatch(/--ui-bg-muted:\s*rgb\(255 255 255 \/ 0\.03\);/)
    expect(micaDark).toMatch(/--ui-bg-elevated:\s*rgb\(255 255 255 \/ 0\.05\);/)
    expect(micaDark).toMatch(/--ui-bg-accented:\s*rgb\(255 255 255 \/ 0\.08\);/)
  })

  it('keeps dark structural strokes and progress tracks visible', () => {
    const dark = ruleBody('.dark')

    expect(dark).toMatch(/--cloudburst-progress-track:\s*rgb\(255 255 255 \/ 0\.16\);/)
    expect(dark).toMatch(/--ui-border:\s*rgb\(255 255 255 \/ 0\.09\);/)
    expect(dark).toMatch(/--ui-border-accented:\s*rgb\(255 255 255 \/ 0\.14\);/)
  })

  it('keeps opaque dark surfaces close to the main background', () => {
    const dark = ruleBody('.dark')

    expect(dark).toMatch(/--ui-bg-muted:\s*color-mix\(in oklab, var\(--cloudburst-bg-dark\) 98%, white\);/)
    expect(dark).toMatch(/--ui-bg-elevated:\s*color-mix\(in oklab, var\(--cloudburst-bg-dark\) 97%, white\);/)
    expect(dark).toMatch(/--ui-bg-accented:\s*color-mix\(in oklab, var\(--cloudburst-bg-dark\) 94%, white\);/)
  })

  it('keeps the Off details panel flat and raises it only over window materials', () => {
    expect(ruleBody(':root')).toMatch(/--cloudburst-detail-surface:\s*transparent;/)
    expect(ruleBody('html.appearance-mica #__nuxt')).toMatch(/--cloudburst-detail-surface:\s*var\(--ui-bg-elevated\);/)
  })

  it('raises paused-name contrast only in full Mica', () => {
    expect(ruleBody(':root')).toMatch(/--cloudburst-paused-name:\s*var\(--ui-text-dimmed\);/)
    expect(ruleBody('html.appearance-mica #__nuxt')).toMatch(/--cloudburst-paused-name:\s*var\(--ui-text-toned\);/)
  })
})
