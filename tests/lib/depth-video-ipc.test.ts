import { invoke } from '@tauri-apps/api/core'
import { describe, expect, it, vi } from 'vitest'
import {
  convertVideoToDepthVideo,
  prepareDepthVideoEngine,
  suggestDepthVideoOutputPath,
} from '@/lib/ipc'

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
      enginePath: 'C:\\tools\\banana-depth-video.exe',
    })

    expect(invoke).toHaveBeenCalledWith('convert_video_to_depth_video', {
      input: {
        sourcePath: 'C:\\tmp\\movie.mp4',
        outputPath: 'C:\\tmp\\movie_depth_08081230.mp4',
        enginePath: 'C:\\tools\\banana-depth-video.exe',
      },
    })
  })

  it('prepares the official local depth-video engine and returns the launcher path', async () => {
    vi.mocked(invoke).mockResolvedValue({
      enginePath: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\depth-video-engine\\banana-depth-video.cmd',
      engineDir: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\depth-video-engine',
      message: '本地深度视频引擎已配置',
    })

    await prepareDepthVideoEngine()

    expect(invoke).toHaveBeenCalledWith('prepare_depth_video_engine')
  })
})
