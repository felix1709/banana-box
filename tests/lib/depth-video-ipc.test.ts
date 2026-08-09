import { invoke } from '@tauri-apps/api/core'
import { describe, expect, it, vi } from 'vitest'
import { convertVideoToDepthVideo, suggestDepthVideoOutputPath } from '@/lib/ipc'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('depth video IPC', () => {
  it('suggests a default depth-video output path', async () => {
    vi.mocked(invoke).mockResolvedValue('C:\\tmp\\movie_depth_08081230.mp4')

    await suggestDepthVideoOutputPath({ sourcePath: 'C:\\tmp\\movie.mp4' })

    expect(invoke).toHaveBeenCalledWith('suggest_depth_video_output_path', {
      input: { sourcePath: 'C:\\tmp\\movie.mp4' },
    })
  })

  it('converts video through the local depth-video command', async () => {
    vi.mocked(invoke).mockResolvedValue({
      outputPath: 'C:\\tmp\\movie_depth_08081230.mp4',
    })

    await convertVideoToDepthVideo({
      sourcePath: 'C:\\tmp\\movie.mp4',
      outputPath: 'C:\\tmp\\movie_depth_08081230.mp4',
    })

    expect(invoke).toHaveBeenCalledWith('convert_video_to_depth_video', {
      input: {
        sourcePath: 'C:\\tmp\\movie.mp4',
        outputPath: 'C:\\tmp\\movie_depth_08081230.mp4',
      },
    })
  })
})
