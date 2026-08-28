const bytesToBase64 = (bytes: Uint8Array) => {
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

export const fileToBase64 = async (file: File) => {
  try {
    return bytesToBase64(new Uint8Array(await file.arrayBuffer()))
  }
  catch {
    throw new Error(`Could not read ${file.name}.`)
  }
}
