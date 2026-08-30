import { readFile, writeFile } from 'node:fs/promises'

const version = process.argv[2]
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/

if (!semver.test(version ?? '')) {
  console.error('Usage: pnpm release:prepare <semver>')
  process.exit(1)
}

for (const path of ['package.json', 'src-tauri/tauri.conf.json']) {
  const contents = await readFile(path, 'utf8')
  await writeFile(path, contents.replace(/("version"\s*:\s*)"[^"]+"/, `$1"${version}"`))
}

const cargoPath = 'src-tauri/Cargo.toml'
const cargoToml = await readFile(cargoPath, 'utf8')
await writeFile(cargoPath, cargoToml.replace(/^(version\s*=\s*)"[^"]+"/m, `$1"${version}"`))

const cargoLockPath = 'src-tauri/Cargo.lock'
const cargoLock = await readFile(cargoLockPath, 'utf8')
await writeFile(
  cargoLockPath,
  cargoLock.replace(
    /(\[\[package\]\]\s+name = "cloudburst"\s+version = )"[^"]+"/m,
    `$1"${version}"`,
  ),
)

console.log(`Prepared Cloudburst ${version}. Review, commit, and tag the changes with v${version}.`)
