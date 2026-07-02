import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const root = new URL('..', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')
const packageJson = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
const version = packageJson.version
const repo = process.env.GITHUB_REPOSITORY || 'felix1709/banana-box'
const tag = `v${version}`
const bundleDir = join(root, 'src-tauri', 'target', 'release', 'bundle')
const setupName = `banana-box_${version}_x64-setup.exe`
const signature = readFileSync(join(bundleDir, 'nsis', `${setupName}.sig`), 'utf8').trim()

const manifest = {
  version,
  notes: `Banana Box ${tag}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature,
      url: `https://github.com/${repo}/releases/download/${tag}/${setupName}`,
    },
  },
}

writeFileSync(join(bundleDir, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`)
console.log(`Generated ${join(bundleDir, 'latest.json')}`)
