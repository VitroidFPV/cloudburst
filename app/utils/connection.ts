export const isLoopbackEndpoint = (endpoint: string) => {
  try {
    const host = new URL(endpoint).hostname.replace(/^\[|\]$/g, '').toLowerCase()
    return host === 'localhost' || host === '::1' || host.startsWith('127.')
  }
  catch {
    return false
  }
}
