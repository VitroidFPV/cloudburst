import { readFile } from 'node:fs/promises'

const packageJson = JSON.parse(await readFile('package.json', 'utf8'))
const tauriConfig = JSON.parse(await readFile('src-tauri/tauri.conf.json', 'utf8'))
const cargoToml = await readFile('src-tauri/Cargo.toml', 'utf8')
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
const cargoLock = await readFile('src-tauri/Cargo.lock', 'utf8')
const cargoLockVersion = cargoLock.match(/\[\[package\]\]\s+name = "cloudburst"\s+version = "([^"]+)"/m)?.[1]

const versions = {
  'package.json': packageJson.version,
  'src-tauri/Cargo.toml': cargoVersion,
  'src-tauri/Cargo.lock': cargoLockVersion,
  'src-tauri/tauri.conf.json': tauriConfig.version,
}

const uniqueVersions = new Set(Object.values(versions))
if (uniqueVersions.size !== 1 || uniqueVersions.has(undefined)) {
  console.error('Release versions do not match:', versions)
  process.exit(1)
}

const version = packageJson.version
const releaseTag = process.env.RELEASE_TAG
if (releaseTag && releaseTag !== `v${version}`) {
  console.error(`Tag ${releaseTag} does not match application version v${version}`)
  process.exit(1)
}

console.log(`Cloudburst version ${version} is consistent.`)
