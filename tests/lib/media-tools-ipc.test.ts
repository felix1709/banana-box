import { invoke } from '@tauri-apps/api/core'
import { describe, expect, it, vi } from 'vitest'
import { prepareFfmpegTools } from '@/lib/ipc'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('media tools IPC', () => {
  it('prepares app-managed FFmpeg tools with a progress operation id', async () => {
    vi.mocked(invoke).mockResolvedValue({
      ffmpegPath: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\ffmpeg\\bin\\ffmpeg.exe',
      ffprobePath: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\ffmpeg\\bin\\ffprobe.exe',
      binDir: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\ffmpeg\\bin',
      message: 'FFmpeg 已配置完成，可以开始压缩视频',
    })

    await prepareFfmpegTools({ operationId: 'op-ffmpeg-1' })

    expect(invoke).toHaveBeenCalledWith('prepare_ffmpeg_tools', {
      input: { operationId: 'op-ffmpeg-1' },
    })
  })
})
