import { describe, expect, it } from 'vitest'
import { isLoopbackEndpoint } from '../../app/utils/connection'

describe('isLoopbackEndpoint', () => {
  it('accepts loopback hosts in every common spelling', () => {
    expect(isLoopbackEndpoint('http://localhost:8080')).toBe(true)
    expect(isLoopbackEndpoint('http://LOCALHOST:8080')).toBe(true)
    expect(isLoopbackEndpoint('http://127.0.0.1:8080')).toBe(true)
    expect(isLoopbackEndpoint('http://127.9.1.1:8080')).toBe(true)
    expect(isLoopbackEndpoint('http://[::1]:8080')).toBe(true)
  })

  it('rejects remote hosts and invalid URLs', () => {
    expect(isLoopbackEndpoint('http://192.168.1.50:8080')).toBe(false)
    expect(isLoopbackEndpoint('https://qbittorrent.example.test')).toBe(false)
    expect(isLoopbackEndpoint('http://localhost.evil.test')).toBe(false)
    expect(isLoopbackEndpoint('not-a-url')).toBe(false)
    expect(isLoopbackEndpoint('')).toBe(false)
  })
})
