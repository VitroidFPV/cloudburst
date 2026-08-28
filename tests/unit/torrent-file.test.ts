import { describe, expect, it } from 'vitest'
import { fileToBase64 } from '../../app/utils/torrent-file'

describe('fileToBase64', () => {
  it('encodes file content as base64', async () => {
    const file = new File([new Uint8Array([104, 101, 108, 108, 111])], 'Debian.torrent', { type: 'application/x-bittorrent' })

    await expect(fileToBase64(file)).resolves.toBe(btoa('hello'))
  })

  it('rejects with a readable error when reading fails', async () => {
    const failing = {
      name: 'broken.torrent',
      arrayBuffer: () => Promise.reject(new Error('disk gone')),
    } as unknown as File

    await expect(fileToBase64(failing)).rejects.toThrow('Could not read broken.torrent.')
  })
})
