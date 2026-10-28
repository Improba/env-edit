import { readFileSync } from 'node:fs'

const tag = process.argv[2]
const packageVersion = JSON.parse(readFileSync('package.json', 'utf8')).version
const lockVersion = JSON.parse(readFileSync('package-lock.json', 'utf8')).version
const tauriVersion = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version
const cargoToml = readFileSync('src-tauri/Cargo.toml', 'utf8')
const cargoVersion = cargoToml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1]
const cargoLock = readFileSync('src-tauri/Cargo.lock', 'utf8')
const cargoLockVersion = cargoLock.match(/^\[\[package\]\]\s*\nname = "envedit"\s*\nversion = "([^"]+)"/m)?.[1]

if (!tag || !/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag)) {
  throw new Error(`Tag invalide : ${tag ?? '(absent)'}`)
}

const expected = tag.slice(1)
for (const [file, version] of [
  ['package.json', packageVersion],
  ['package-lock.json', lockVersion],
  ['src-tauri/tauri.conf.json', tauriVersion],
  ['src-tauri/Cargo.toml', cargoVersion],
  ['src-tauri/Cargo.lock', cargoLockVersion],
]) {
  if (version !== expected) {
    throw new Error(`${file} : version ${version ?? '(absente)'}, attendue ${expected}`)
  }
}

console.log(`Versions cohérentes pour ${tag}`)
