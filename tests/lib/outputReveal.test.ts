import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { outputFolderPath, revealOutputPath } from '@/lib/outputReveal'

vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn(),
  revealItemInDir: vi.fn(),
}))

describe('output reveal helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('gets the parent folder for a Windows output path', () => {
    expect(outputFolderPath('C:\\Users\\admin\\Desktop\\movie_depth.mp4')).toBe(
      'C:\\Users\\admin\\Desktop',
    )
  })

  it('gets the parent folder for a POSIX output path', () => {
    expect(outputFolderPath('/Users/admin/Desktop/movie_depth.mp4')).toBe('/Users/admin/Desktop')
  })

  it('reveals the generated file in the system file manager', async () => {
    vi.mocked(revealItemInDir).mockResolvedValue()

    await expect(revealOutputPath('C:\\Users\\admin\\Desktop\\movie.mp4')).resolves.toBe(true)

    expect(revealItemInDir).toHaveBeenCalledWith('C:\\Users\\admin\\Desktop\\movie.mp4')
    expect(openPath).not.toHaveBeenCalled()
  })

  it('opens the parent folder when selecting the generated file fails', async () => {
    vi.mocked(revealItemInDir).mockRejectedValue(new Error('unsupported'))
    vi.mocked(openPath).mockResolvedValue()

    await expect(revealOutputPath('C:\\Users\\admin\\Desktop\\movie.mp4')).resolves.toBe(true)

    expect(openPath).toHaveBeenCalledWith('C:\\Users\\admin\\Desktop')
  })

  it('returns false when both reveal and folder open fail', async () => {
    vi.mocked(revealItemInDir).mockRejectedValue(new Error('unsupported'))
    vi.mocked(openPath).mockRejectedValue(new Error('blocked'))

    await expect(revealOutputPath('C:\\Users\\admin\\Desktop\\movie.mp4')).resolves.toBe(false)
  })
})
