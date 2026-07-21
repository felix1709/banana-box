import { copyFile, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawn } from 'node:child_process'

const PI_WEB_VERSION = '0.7.16'
const __dirname = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(__dirname, '..')
const resourcesDir = join(projectRoot, 'src-tauri', 'resources')
const runtimeDir = join(resourcesDir, 'pi-web')
const archivePath = join(resourcesDir, 'pi-web-runtime.zip')
const nodeDir = join(runtimeDir, 'node')
const manifestPath = join(runtimeDir, 'banana-pi-web-runtime.json')
const nodeTarget = join(nodeDir, process.platform === 'win32' ? 'node.exe' : 'node')

async function readManifest() {
  try {
    return JSON.parse(await readFile(manifestPath, 'utf8'))
  } catch {
    return null
  }
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const executable = process.platform === 'win32' && command === 'npm' ? 'npm.cmd' : command
    const child = spawn(executable, args, {
      cwd: projectRoot,
      stdio: 'inherit',
      ...options,
    })
    child.on('error', reject)
    child.on('exit', (code) => {
      if (code === 0) resolve()
      else reject(new Error(`${command} ${args.join(' ')} exited with code ${code}`))
    })
  })
}

async function main() {
  const manifest = await readManifest()
  const packageRoot = join(runtimeDir, 'node_modules', '@agegr', 'pi-web')
  const scriptPath = join(packageRoot, 'bin', 'pi-web.js')
  const runtimeReady =
    manifest?.piWebVersion === PI_WEB_VERSION &&
    manifest?.platform === process.platform &&
    existsSync(scriptPath) &&
    existsSync(nodeTarget)

  if (runtimeReady && existsSync(archivePath)) {
    console.log(`PI-Web runtime ${PI_WEB_VERSION} already prepared.`)
    return
  }

  await rm(archivePath, { force: true })
  await mkdir(resourcesDir, { recursive: true })

  if (!runtimeReady) {
    await rm(runtimeDir, { recursive: true, force: true })
    await mkdir(runtimeDir, { recursive: true })
    await mkdir(nodeDir, { recursive: true })

    await writeFile(
      join(runtimeDir, 'package.json'),
      JSON.stringify(
        {
          private: true,
          dependencies: {
            '@agegr/pi-web': PI_WEB_VERSION,
          },
        },
        null,
        2,
      ),
    )

    await run('npm', ['install', '--omit=dev', '--no-audit', '--fund=false', '--prefix', runtimeDir])
    await copyFile(process.execPath, nodeTarget)

    await writeFile(
      manifestPath,
      JSON.stringify(
        {
          piWebVersion: PI_WEB_VERSION,
          platform: process.platform,
          arch: process.arch,
          nodeSource: process.execPath,
          preparedAt: new Date().toISOString(),
        },
        null,
        2,
      ),
    )
  }

  await writeFile(join(runtimeDir, '.gitkeep'), '')
  await run('tar', ['-a', '-cf', archivePath, '-C', runtimeDir, '.'])

  console.log(`Prepared PI-Web runtime ${PI_WEB_VERSION} in ${runtimeDir}`)
  console.log(`Packed PI-Web runtime archive at ${archivePath}`)
}

main().catch((error) => {
  console.error(error)
  process.exit(1)
})
