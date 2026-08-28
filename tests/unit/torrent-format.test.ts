import { afterEach, describe, expect, it, vi } from 'vitest'
import { formatAddedOn, formatBytes, formatEta, formatSpeed } from '../../app/utils/torrent-format'

describe('torrent formatting', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('formats byte counts and transfer speeds', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(1024)).toBe('1.0 KB')
    expect(formatBytes(5 * 1024 ** 3)).toBe('5.00 GB')
    expect(formatSpeed(0)).toBe('—')
    expect(formatSpeed(1536)).toBe('1.5 KB/s')
  })

  it('formats short relative added times', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-08-28T12:00:00Z'))
    const now = Math.floor(Date.now() / 1000)

    expect(formatAddedOn(now - 30)).toBe('Just now')
    expect(formatAddedOn(now - 5 * 60)).toBe('5m ago')
    expect(formatAddedOn(now - 2 * 60 * 60)).toBe('2h ago')
  })

  it('uses status-aware ETA labels and finite durations', () => {
    expect(formatEta(null, 'paused')).toBe('Paused')
    expect(formatEta(null, 'checking')).toBe('Checking')
    expect(formatEta(null, 'stalled')).toBe('Waiting')
    expect(formatEta(null, 'downloading')).toBe('∞')
    expect(formatEta(3_661, 'downloading')).toBe('1h 1m')
    expect(formatEta(125, 'downloading')).toBe('2m 5s')
  })
})
